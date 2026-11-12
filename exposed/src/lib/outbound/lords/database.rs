use super::relational::{CategoryRow, InterestRow, ProfileRow, position, reconstruct};
use crate::domain::models::lords::SourceTimestamp;
use chrono::{DateTime, Utc};

use crate::{
    domain::{
        models::{
            entity_search::EntitySearchRequest,
            lords::{
                CompleteLordsCapture, LordProfile, LordRegister, LordsError, LordsLoadOutcome,
            },
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
    async fn load_lords(
        &self,
        capture: &CompleteLordsCapture,
    ) -> Result<LordsLoadOutcome, LordsError> {
        let digest = super::storage::capture_digest(capture)?;
        let captured_at =
            time::OffsetDateTime::from_unix_timestamp(capture.captured_at().timestamp())
                .and_then(|date| {
                    date.replace_nanosecond(capture.captured_at().timestamp_subsec_nanos())
                })
                .map_err(|error| LordsError::Capture(error.to_string()))?;
        let member_count = i64::try_from(capture.members().len())
            .map_err(|error| LordsError::Capture(error.to_string()))?;
        let mut tx = self.pool().begin().await?;
        let inserted = sqlx::query!(
            "INSERT INTO exposed.lords_imports (ingestion_key, capture_digest, captured_at, member_count)
             VALUES ($1, $2, $3, $4) ON CONFLICT (ingestion_key) DO NOTHING RETURNING ingestion_key",
            capture.key().uuid(), digest, captured_at, member_count
        ).fetch_optional(&mut *tx).await?;
        if inserted.is_none() {
            let previous = sqlx::query!(
                "SELECT capture_digest FROM exposed.lords_imports WHERE ingestion_key = $1",
                capture.key().uuid()
            )
            .fetch_one(&mut *tx)
            .await?;
            if previous.capture_digest != digest {
                return Err(LordsError::Conflict);
            }
            tx.commit().await?;
            return Ok(LordsLoadOutcome::AlreadyLoaded);
        }
        let mut members: Vec<_> = capture.members().iter().collect();
        members.sort_by_key(|member| member.register().member.id);
        for member in members {
            let profile = &member.register().member;
            let member_id = crate::outbound::member_identity::acquire(&mut tx, profile.id).await?;
            let won = sqlx::query!(
                "INSERT INTO exposed.member_profiles
                 (member_id, chamber, name, party_name, membership_from, membership_status, membership_end_date,
                  source_document, captured_at, ingestion_key)
                 VALUES ($1, 'lords', $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (member_id, chamber) DO UPDATE SET
                     name = EXCLUDED.name, party_name = EXCLUDED.party_name,
                     membership_from = EXCLUDED.membership_from,
                     membership_status = EXCLUDED.membership_status,
                     membership_end_date = EXCLUDED.membership_end_date,
                     source_document = EXCLUDED.source_document,
                     captured_at = EXCLUDED.captured_at, ingestion_key = EXCLUDED.ingestion_key
                 WHERE (exposed.member_profiles.captured_at, exposed.member_profiles.ingestion_key)
                     < (EXCLUDED.captured_at, EXCLUDED.ingestion_key)
                 RETURNING member_id",
                member_id, profile.name, profile.party_name, profile.membership_from,
                profile.membership_status,
                profile.membership_end_date.as_ref().map(SourceTimestamp::as_str),
                member.raw(), captured_at, capture.key().uuid()
            ).fetch_optional(&mut *tx).await?;
            if won.is_none() {
                continue;
            }
            replace_tree(&mut tx, member_id, member.register()).await?;
        }
        tx.commit().await?;
        Ok(LordsLoadOutcome::Loaded)
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
            "SELECT p.member_id, m.parliament_member_id, p.name, p.party_name,
                    p.membership_from, p.membership_status, p.membership_end_date, p.captured_at AS \"captured_at!\",
                    exposed.word_similarity($1, p.name) AS similarity
             FROM exposed.member_profiles p JOIN exposed.members m ON m.id = p.member_id AND p.chamber = 'lords'
             WHERE p.name OPERATOR(exposed.%>) $1 AND exposed.word_similarity($1, p.name) >= $2
             ORDER BY similarity DESC, m.parliament_member_id LIMIT $3",
            request.term(),
            request.strictness(),
            i64::from(request.max_entries())
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
            "SELECT p.member_id, m.parliament_member_id, p.name, p.party_name,
                    p.membership_from, p.membership_status, p.membership_end_date, p.captured_at AS \"captured_at!\",
                    NULL::real AS similarity
             FROM exposed.member_profiles p JOIN exposed.members m ON m.id = p.member_id AND p.chamber = 'lords'
             WHERE m.parliament_member_id = $1",
            id
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let categories =
            sqlx::query_as!(CategoryRow,
            "SELECT category_position, source_id, name, sort_order FROM exposed.lords_categories
             WHERE member_id = $1 ORDER BY category_position", row.member_id
        )
            .fetch_all(&mut *tx)
            .await?;
        let interests = sqlx::query_as!(InterestRow,
            "SELECT category_position, node_position, parent_position, sibling_position, source_id,
                    interest_text, created_when, last_amended_when, deleted_when, is_correction
             FROM exposed.lords_interests WHERE member_id = $1 ORDER BY category_position, node_position",
             row.member_id
        ).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some(LordRegister {
            captured_at: capture_date(row.captured_at)?,
            member: row.profile()?,
            categories: reconstruct(categories, interests)?,
        }))
    }
}

fn capture_date(date: time::OffsetDateTime) -> Result<DateTime<Utc>, LordsError> {
    DateTime::from_timestamp(date.unix_timestamp(), date.nanosecond())
        .ok_or_else(|| LordsError::Database("stored capture timestamp is out of range".into()))
}

async fn replace_tree(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    member_id: uuid::Uuid,
    register: &LordRegister,
) -> Result<(), LordsError> {
    sqlx::query!(
        "DELETE FROM exposed.lords_categories WHERE member_id = $1",
        member_id
    )
    .execute(&mut **tx)
    .await?;
    for (category_position, category) in register.categories.iter().enumerate() {
        let category_position = position(category_position)?;
        sqlx::query!(
            "INSERT INTO exposed.lords_categories (member_id, category_position, source_id, name, sort_order)
             VALUES ($1, $2, $3, $4, $5)",
            member_id, category_position, i64::from(category.source_id.value()), category.name, category.sort_order
        ).execute(&mut **tx).await?;
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
                "INSERT INTO exposed.lords_interests
                 (member_id, category_position, node_position, parent_position, sibling_position,
                  source_id, interest_text, created_when, last_amended_when, deleted_when, is_correction)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                member_id, category_position, node_position, parent_position, position(sibling)?,
                i64::from(interest.source_id.value()), interest.text,
                interest.created_when.as_ref().map(SourceTimestamp::as_str),
                interest.last_amended_when.as_ref().map(SourceTimestamp::as_str),
                interest.deleted_when.as_ref().map(SourceTimestamp::as_str), interest.is_correction
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
