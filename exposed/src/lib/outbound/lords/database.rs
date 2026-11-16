use super::relational::{InterestRow, ProfileRow, position, reconstruct};
use crate::domain::models::lords::SourceTimestamp;

use crate::{
    domain::{
        models::{
            entity_search::EntitySearchRequest,
            lords::{CompleteLordsCapture, LordProfile, LordRegister, LordsError},
            membership::Chamber,
            parliament_member::MemberId,
            search_similarity::SearchSimilarity,
        },
        repositories::lords::LordsRepository,
    },
    outbound::postgres::{ExposedDatabase, word_similarity_candidate_threshold},
};

impl From<sqlx::Error> for LordsError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error.to_string())
    }
}

impl LordsRepository for ExposedDatabase {
    async fn load_lords(&self, capture: &CompleteLordsCapture) -> Result<(), LordsError> {
        let fetched_at = time::OffsetDateTime::from_unix_timestamp_nanos(
            i128::from(capture.captured_at().timestamp()) * 1_000_000_000
                + i128::from(capture.captured_at().timestamp_subsec_nanos()),
        )
        .map_err(|error| LordsError::Capture(error.to_string()))?;
        let mut tx = self.pool().begin().await?;
        for register in capture.members() {
            let member_id =
                crate::outbound::member_identity::upsert(&mut tx, &register.member).await?;
            replace_tree(&mut tx, member_id, register, fetched_at).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn search_lords(
        &self,
        request: &EntitySearchRequest,
    ) -> Result<Vec<(LordProfile, SearchSimilarity)>, LordsError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query!(
            "SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)",
            word_similarity_candidate_threshold(request.strictness())
        )
        .fetch_one(&mut *tx)
        .await?;
        let rows = sqlx::query_as!(
            ProfileRow,
            "SELECT m.id AS member_id, m.parliament_member_id, m.name, m.parliamentary_name, m.party_name, m.party_id, k.name AS membership_kind, m.membership_start_date,
                    m.membership_from, m.membership_status, m.membership_end_date,
                    GREATEST(exposed.word_similarity($1, m.name), exposed.word_similarity($1, m.parliamentary_name), exposed.word_similarity($1, m.formal_name)) AS similarity
             FROM exposed.members m JOIN exposed.membership_kinds k ON k.id = m.membership_kind
             JOIN exposed.chambers c ON c.id = k.chamber_id
             WHERE c.name = $4 AND (m.name OPERATOR(exposed.%>) $1 OR m.parliamentary_name OPERATOR(exposed.%>) $1 OR m.formal_name OPERATOR(exposed.%>) $1) AND GREATEST(exposed.word_similarity($1, m.name), exposed.word_similarity($1, m.parliamentary_name), exposed.word_similarity($1, m.formal_name)) >= $2
             ORDER BY similarity DESC, m.parliament_member_id LIMIT $3",
            request.term(),
            request.strictness(),
            i64::from(request.max_entries()),
            Chamber::Lords.name()
        )
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        rows.into_iter()
            .map(|row| {
                let similarity = SearchSimilarity::from(row.similarity.unwrap_or(0.0));
                Ok((row.profile()?, similarity))
            })
            .collect()
    }
}

impl ExposedDatabase {
    pub(crate) async fn lord_register(
        &self,
        id: MemberId,
    ) -> Result<Option<LordRegister>, LordsError> {
        let id =
            i32::try_from(id.value()).map_err(|error| LordsError::Database(error.to_string()))?;
        let mut tx = self.pool().begin().await?;
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        let row = sqlx::query_as!(
            ProfileRow,
            "SELECT m.id AS member_id, m.parliament_member_id, m.name, m.parliamentary_name, m.party_name, m.party_id, k.name AS membership_kind, m.membership_start_date,
                    m.membership_from, m.membership_status, m.membership_end_date,
                    NULL::real AS similarity
             FROM exposed.members m JOIN exposed.membership_kinds k ON k.id = m.membership_kind
             JOIN exposed.chambers c ON c.id = k.chamber_id
             WHERE c.name = $2 AND m.parliament_member_id = $1",
            id,
            Chamber::Lords.name()
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let interests = sqlx::query_as!(InterestRow,
            "SELECT category_position, category_source_id, category_name, category_sort_order, node_position, parent_position, sibling_position, source_id,
                    declaration_text, created_when, last_amended_when, deleted_when, is_correction
             FROM exposed.simple_declarations WHERE member_id = $1 ORDER BY category_position, node_position",
             row.member_id
        ).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some(LordRegister {
            member: row.profile()?,
            categories: reconstruct(interests)?,
        }))
    }
}

async fn replace_tree(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    member_id: uuid::Uuid,
    register: &LordRegister,
    fetched_at: time::OffsetDateTime,
) -> Result<(), LordsError> {
    sqlx::query!(
        "DELETE FROM exposed.simple_declarations WHERE member_id = $1",
        member_id
    )
    .execute(&mut **tx)
    .await?;
    for (category_position, category) in register.categories.iter().enumerate() {
        let category_position = position(category_position)?;
        let mut pending: Vec<_> = category
            .interests
            .iter()
            .enumerate()
            .rev()
            .map(|(sibling, interest)| (None::<i64>, sibling, interest))
            .collect();
        let mut node_position = 0_i64;
        while let Some((parent_position, sibling, interest)) = pending.pop() {
            sqlx::query!(
                "INSERT INTO exposed.simple_declarations
                 (member_id, category_position, node_position, parent_position, sibling_position,
                  source_id, declaration_text, created_when, last_amended_when, deleted_when, is_correction, category_source_id, category_name, category_sort_order, fetched_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
                member_id, category_position, node_position, parent_position, position(sibling)?,
                i64::from(interest.source_id.value()), interest.text,
                interest.created_when.as_ref().map(SourceTimestamp::as_str),
                interest.last_amended_when.as_ref().map(SourceTimestamp::as_str),
                interest.deleted_when.as_ref().map(SourceTimestamp::as_str), interest.is_correction, i64::from(category.source_id.value()), category.name, category.sort_order, fetched_at
            ).execute(&mut **tx).await?;
            pending.extend(
                interest
                    .children
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(sibling, child)| (Some(node_position), sibling, child)),
            );
            node_position += 1;
        }
    }
    Ok(())
}
