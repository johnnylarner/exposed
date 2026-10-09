//! Concrete implementation of the repository

use std::str::FromStr;

use crate::{
    domain::{
        models::{
            entity_search::{EntitySearchRequest, FunderSearchMatch, SearchMatchSource},
            funder::{Funder, FunderId, FunderKind},
            search_similarity::SearchSimilarity,
        },
        repositories::funder_repository::{FunderRepo, FunderRepoError},
    },
    outbound::postgres::{ExposedDatabase, word_similarity_candidate_threshold},
};

impl FunderRepo for ExposedDatabase {
    async fn get_funders_by_text_search_score(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<(FunderSearchMatch, SearchSimilarity)>, FunderRepoError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        sqlx::query!(
            "SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)",
            word_similarity_candidate_threshold(req.strictness())
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        let rows = sqlx::query!(
            "
            WITH candidates AS (
                SELECT id FROM exposed.funders WHERE funder_name %> $1
                UNION
                SELECT funder_id FROM exposed.funder_aliases WHERE funder_alias %> $1
            ), scored AS (
                SELECT f.id, f.funder_name AS name, f.funder_kind AS kind,
                    word_similarity($1, f.funder_name) AS canonical_score,
                    a.funder_alias, a.alias_score
                FROM candidates c
                JOIN exposed.funders f ON f.id = c.id
                LEFT JOIN LATERAL (
                    SELECT funder_alias, word_similarity($1, funder_alias) AS alias_score
                    FROM exposed.funder_aliases WHERE funder_id = f.id
                    ORDER BY alias_score DESC, funder_alias COLLATE \"C\" ASC
                    LIMIT 1
                ) a ON true
            )
            SELECT id, name, kind,
                CASE WHEN alias_score > canonical_score THEN funder_alias END AS matched_alias,
                GREATEST(canonical_score, COALESCE(alias_score, 0)) AS similarity_score
            FROM scored
            WHERE GREATEST(canonical_score, COALESCE(alias_score, 0)) >= $2
            ORDER BY similarity_score DESC, id
            LIMIT $3
            ",
            req.term(),
            req.strictness(),
            i64::from(req.max_entries()),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                let kind = r.kind.map_or(FunderKind::NotSpecified, |ref kind| {
                    FunderKind::from_str(kind).unwrap()
                });
                let funder = Funder::new(FunderId::new(r.id), r.name, kind);
                let score = SearchSimilarity::from(r.similarity_score.unwrap_or(0_f32));
                Ok((
                    FunderSearchMatch::new(
                        funder,
                        r.matched_alias.map_or(SearchMatchSource::Name, |name| {
                            SearchMatchSource::Alias { name }
                        }),
                    ),
                    score,
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod scoring {
    use sqlx::PgPool;

    use crate::{
        domain::{
            models::entity_search::EntitySearchRequest, repositories::funder_repository::FunderRepo,
        },
        outbound::postgres::ExposedDatabase,
    };

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures(
            "../../../../db/fixtures/add_members.sql",
            "../../../../db/fixtures/add_declarations_and_funding_entries.sql"
        )
    )]
    async fn orders_correctly(pool: PgPool) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool);

        let term = EntitySearchRequest::new_with_strictness("McDonald's".into(), 5, 0_f32).unwrap();
        let results = db.get_funders_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.first().unwrap().0.funder().name(), "McDonald's");

        Ok(())
    }

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures(
            "../../../../db/fixtures/add_members.sql",
            "../../../../db/fixtures/add_declarations_and_funding_entries.sql"
        )
    )]
    async fn obeys_max_entries(pool: PgPool) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool);

        let term = EntitySearchRequest::new_with_strictness("McDonald's".into(), 1, 0_f32).unwrap();
        let results = db.get_funders_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().0.funder().name(), "McDonald's");

        Ok(())
    }
    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures(
            "../../../../db/fixtures/add_members.sql",
            "../../../../db/fixtures/add_declarations_and_funding_entries.sql"
        )
    )]
    async fn filters_similarity(pool: PgPool) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool);

        let term =
            EntitySearchRequest::new_with_strictness("Aaron Banks".into(), 5, 1_f32).unwrap();
        let results = db.get_funders_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().0.funder().name(), "Aaron Banks");

        Ok(())
    }
}

#[cfg(test)]
mod alias_scoring {
    use crate::{
        domain::{
            models::entity_search::{EntitySearchRequest, SearchMatchSource},
            repositories::funder_repository::FunderRepo,
        },
        outbound::postgres::ExposedDatabase,
    };
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../db/migrations")]
    async fn provenance_thresholds_and_identity_limit(pool: PgPool) -> sqlx::Result<()> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with(pool.connect_options().as_ref().clone())
            .await?;
        let first = uuid::Uuid::from_u128(1);
        let second = uuid::Uuid::from_u128(2);
        sqlx::query!("INSERT INTO exposed.funders (id, funder_name) VALUES ($1, 'Unite the Union'), ($2, 'West Midlands')", first, second).execute(&pool).await?;
        for alias in [
            "West Midlands",
            "WEST MIDLANDS",
            "West Midlands Union",
            "West Midlands Branch",
        ] {
            sqlx::query!(
                "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2)",
                first,
                alias
            )
            .execute(&pool)
            .await?;
        }
        sqlx::query!("INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, 'West Midlands')", second).execute(&pool).await?;
        let before = sqlx::query!(
            "SELECT current_setting('pg_trgm.word_similarity_threshold') AS threshold"
        )
        .fetch_one(&pool)
        .await?
        .threshold;
        let db = ExposedDatabase::from(pool.clone());
        let request = |term: &str, limit, threshold| {
            EntitySearchRequest::new_with_strictness(term.into(), limit, threshold).unwrap()
        };
        let rows = db
            .get_funders_by_text_search_score(&request("West Midlands", 2, 1.0))
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0.funder().id().value(), first);
        assert_eq!(
            rows[0].0.source(),
            &SearchMatchSource::Alias {
                name: "WEST MIDLANDS".into()
            }
        );
        assert_eq!(rows[1].0.source(), &SearchMatchSource::Name);
        assert_eq!(rows[0].1.value().to_bits(), 1.0_f32.to_bits());
        let rows = db
            .get_funders_by_text_search_score(&request("Unite the Union", 2, 1.0))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.source(), &SearchMatchSource::Name);
        assert_eq!(
            db.get_funders_by_text_search_score(&request("zzzzzz", 2, 0.0))
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            db.get_funders_by_text_search_score(&request("West Midlands", 1, 1.0))
                .await
                .unwrap()
                .len(),
            1
        );
        let boundary = sqlx::query!("SELECT word_similarity('West Mid', 'West Midlands') AS score")
            .fetch_one(&pool)
            .await?
            .score
            .unwrap();
        assert!(boundary > 0.0 && boundary < 1.0);
        assert_eq!(
            db.get_funders_by_text_search_score(&request("West Mid", 2, boundary))
                .await
                .unwrap()
                .len(),
            2
        );
        let after = sqlx::query!(
            "SELECT current_setting('pg_trgm.word_similarity_threshold') AS threshold"
        )
        .fetch_one(&pool)
        .await?
        .threshold;
        assert_eq!(before, after);
        Ok(())
    }
}
