//! Concrete implementation of the repository

use crate::{
    domain::{
        models::{
            declaration_ingestion::StoredMember, entity_search::EntitySearchRequest,
            parliament_member::ParliamentMember, search_similarity::SearchSimilarity,
        },
        repositories::parliament_member_repository::{
            ParliamentMemberRepo, ParliamentMemberRepoError,
        },
    },
    outbound::postgres::ExposedDatabase,
};

impl ParliamentMemberRepo for ExposedDatabase {
    async fn get_stored_members(&self) -> Result<Vec<StoredMember>, ParliamentMemberRepoError> {
        sqlx::query!(
            "SELECT id, parliament_member_id FROM exposed.members ORDER BY parliament_member_id, id"
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?
        .into_iter()
        .map(|row| {
            let parliament_id = u32::try_from(row.parliament_member_id)
                .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?;
            StoredMember::new(row.id, parliament_id)
                .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))
        })
        .collect()
    }

    #[allow(clippy::cast_sign_loss)]
    async fn get_members_by_text_search_score(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError> {
        sqlx::query!(
            "
            SELECT  
                m.name,
                m.parliament_member_id,
                m.party_name,
                m.party_id,
                m.latest_membership_from as constituency,
                 word_similarity($1, m.name) as similarity_score,
                row_number() OVER (ORDER BY word_similarity($1, m.name) DESC) as rank
            FROM members m
            WHERE word_similarity($1, m.name) >= $2
            ORDER BY similarity_score DESC
            ",
            req.term(),
            req.strictness()
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?
        .into_iter()
        .take_while(|r| {
            r.rank.unwrap_or_else(|| i64::from(req.max_entries() + 1))
                <= i64::from(req.max_entries())
        })
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
