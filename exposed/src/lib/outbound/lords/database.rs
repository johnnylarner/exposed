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
            let id = i32::try_from(member.register().member.id.value())
                .map_err(|error| LordsError::Capture(error.to_string()))?;
            sqlx::query!(
                "INSERT INTO exposed.lords_members (parliament_member_id, name, source_document, captured_at, ingestion_key)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (parliament_member_id) DO UPDATE SET
                     name = EXCLUDED.name, source_document = EXCLUDED.source_document,
                     captured_at = EXCLUDED.captured_at, ingestion_key = EXCLUDED.ingestion_key
                 WHERE (exposed.lords_members.captured_at, exposed.lords_members.ingestion_key)
                     < (EXCLUDED.captured_at, EXCLUDED.ingestion_key)",
                id, member.register().member.name, member.raw(), captured_at, capture.key().uuid()
            ).execute(&mut *tx).await?;
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
        let rows =
            sqlx::query!(
            "SELECT source_document, captured_at, exposed.word_similarity($1, name) AS similarity
             FROM exposed.lords_members
             WHERE name OPERATOR(exposed.%>) $1 AND exposed.word_similarity($1, name) >= $2
             ORDER BY similarity DESC, parliament_member_id LIMIT $3",
            request.term(), request.strictness(), i64::from(request.max_entries())
        )
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;
        rows.into_iter()
            .map(|row| {
                let register = LordRegister::from_source(
                    &row.source_document,
                    capture_date(row.captured_at)?,
                )?;
                Ok((
                    register.member,
                    SearchSimilarity::from(row.similarity.unwrap_or(0.0)),
                ))
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
        let row = sqlx::query!("SELECT source_document, captured_at FROM exposed.lords_members WHERE parliament_member_id = $1", id)
            .fetch_optional(self.pool()).await?;
        row.map(|row| {
            LordRegister::from_source(&row.source_document, capture_date(row.captured_at)?)
        })
        .transpose()
    }
}

fn capture_date(date: time::OffsetDateTime) -> Result<DateTime<Utc>, LordsError> {
    DateTime::from_timestamp(date.unix_timestamp(), date.nanosecond())
        .ok_or_else(|| LordsError::Database("stored capture timestamp is out of range".into()))
}
