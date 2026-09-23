//! Concrete implementation of the repository

use crate::{
    domain::{
        models::{
            entity_search::EntitySearchRequest, parliament_member::ParliamentMember,
            search_similarity::SearchSimilarity,
        },
        repositories::parliament_member_repository::{
            ParliamentMemberRepo, ParliamentMemberRepoError,
        },
    },
    outbound::postgres::ExposedDatabase,
};

impl ParliamentMemberRepo for ExposedDatabase {
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
            r.rank.unwrap_or((req.max_entries() + 1) as i64) <= req.max_entries() as i64
        })
        .map(|r| {
            let member = ParliamentMember::new(
                r.name,
                r.parliament_member_id as usize,
                r.party_name,
                r.constituency,
            );
            let score = SearchSimilarity::from(r.similarity_score.unwrap_or(0_f32));
            Ok((member, score))
        })
        .collect()
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
