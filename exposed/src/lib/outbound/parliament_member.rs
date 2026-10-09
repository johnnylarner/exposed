//! Concrete implementation of the repository

use crate::{
    domain::{
        models::{
            declaration_ingestion::MemberAsId, entity_search::EntitySearchRequest,
            parliament_member::ParliamentMember, search_similarity::SearchSimilarity,
        },
        repositories::parliament_member_repository::{
            ParliamentMemberRepo, ParliamentMemberRepoError,
        },
    },
    outbound::postgres::{ExposedDatabase, word_similarity_candidate_threshold},
};

impl ParliamentMemberRepo for ExposedDatabase {
    async fn get_stored_member_ids(&self) -> Result<Vec<MemberAsId>, ParliamentMemberRepoError> {
        sqlx::query!(
            "
            SELECT id, parliament_member_id
            FROM exposed.members
            ORDER BY parliament_member_id, id
            "
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?
        .into_iter()
        .map(|row| {
            let parliament_id = u32::try_from(row.parliament_member_id)
                .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
            MemberAsId::new(row.id, parliament_id)
                .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))
        })
        .collect()
    }

    #[allow(clippy::cast_sign_loss)]
    async fn get_members_by_text_search_score(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        sqlx::query!(
            "SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)",
            word_similarity_candidate_threshold(req.strictness())
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        let rows = sqlx::query!(
            "
            SELECT  
                m.name,
                m.parliament_member_id,
                m.party_name,
                m.party_id,
                m.latest_membership_from as constituency,
                 word_similarity($1, m.name) as similarity_score
            FROM exposed.members m
            WHERE m.name %> $1 AND word_similarity($1, m.name) >= $2
            ORDER BY similarity_score DESC, m.id
            LIMIT $3
            ",
            req.term(),
            req.strictness(),
            i64::from(req.max_entries())
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                let member = ParliamentMember::new(
                    r.name,
                    r.parliament_member_id as u32,
                    r.party_name,
                    r.party_id as u32,
                    r.constituency,
                );
                let score = SearchSimilarity::from(r.similarity_score.unwrap_or(0_f32));
                Ok((member, score))
            })
            .collect()
    }

    async fn upsert_members(
        &self,
        members: &[ParliamentMember],
    ) -> Result<(), ParliamentMemberRepoError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;

        for m in members {
            let res = sqlx::query!(
                "
            INSERT INTO 
                exposed.members (
                    parliament_member_id, 
                    name, 
                    party_id,
                    party_name,
                    latest_house,
                    latest_membership_from,
                    is_current_commons
           ) 
            VALUES ($1,$2,$3,$4,1,$5,true)
           ON CONFLICT (parliament_member_id) DO UPDATE SET
               name = EXCLUDED.name, party_id = EXCLUDED.party_id,
               party_name = EXCLUDED.party_name, latest_house = EXCLUDED.latest_house,
               latest_membership_from = EXCLUDED.latest_membership_from,
               is_current_commons = EXCLUDED.is_current_commons
           RETURNING id
            ",
                m.member_id().cast_signed(),
                m.name(),
                m.party_id().cast_signed(),
                m.party_name(),
                m.constituency()
            )
            .fetch_one(&mut *tx)
            .await;

            if let Err(e) = res {
                println!("{e}");
            }
        }
        tx.commit()
            .await
            .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod scoring {
    use sqlx::PgPool;

    use crate::{
        domain::{
            models::entity_search::EntitySearchRequest,
            repositories::parliament_member_repository::ParliamentMemberRepo,
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
        let results = db.get_members_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.first().unwrap().0.name(), "John McDonnell");

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
        let results = db.get_members_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().0.name(), "John McDonnell");

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

        let term = EntitySearchRequest::new_strict("John McDonnell".into(), 5).unwrap();
        let results = db.get_members_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().0.name(), "John McDonnell");

        Ok(())
    }
}

#[cfg(test)]
mod threshold_scoring {
    use crate::{
        domain::{
            models::entity_search::EntitySearchRequest,
            repositories::parliament_member_repository::ParliamentMemberRepo,
        },
        outbound::postgres::ExposedDatabase,
    };
    use sqlx::PgPool;

    #[sqlx::test(
        migrations = "../db/migrations",
        fixtures("../../../../db/fixtures/add_members.sql")
    )]
    async fn inclusive_thresholds_are_local(pool: PgPool) -> sqlx::Result<()> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with(pool.connect_options().as_ref().clone())
            .await?;
        sqlx::query!("SELECT word_similarity('initialize', 'initialize')")
            .fetch_one(&pool)
            .await?;
        let initial = sqlx::query!(
            "SELECT current_setting('pg_trgm.word_similarity_threshold') AS threshold"
        )
        .fetch_one(&pool)
        .await?
        .threshold;
        let db = ExposedDatabase::from(pool.clone());
        let boundary =
            sqlx::query!("SELECT word_similarity('John McD', 'John McDonnell') AS score")
                .fetch_one(&pool)
                .await?
                .score
                .unwrap();
        assert!(boundary > 0.0 && boundary < 1.0);
        for (term, threshold, count) in [
            ("zzzzzz", 0.0, 2),
            ("John McDonnell", 1.0, 1),
            ("John McD", boundary, 1),
        ] {
            let req = EntitySearchRequest::new_with_strictness(term.into(), 5, threshold).unwrap();
            let results = db.get_members_by_text_search_score(&req).await.unwrap();
            assert_eq!(results.len(), count);
            let actual = sqlx::query!(
                "SELECT current_setting('pg_trgm.word_similarity_threshold') AS threshold"
            )
            .fetch_one(&pool)
            .await?
            .threshold;
            assert_eq!(actual, initial);
        }
        Ok(())
    }
}
