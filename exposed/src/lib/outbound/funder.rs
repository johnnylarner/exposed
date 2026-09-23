//! Concrete implementation of the repository

use std::str::FromStr;

use crate::{
    domain::{
        models::{
            entity_search::EntitySearchRequest,
            funder::{CompanyFunder, Funder, FunderKind, IndividualFunder},
            search_similarity::SearchSimilarity,
        },
        repositories::funder_repository::{FunderRepo, FunderRepoError},
    },
    outbound::postgres::ExposedDatabase,
};

impl FunderRepo for ExposedDatabase {
    async fn get_funders_by_text_search_score(
        &self,
        req: &EntitySearchRequest,
    ) -> Result<Vec<(Funder, SearchSimilarity)>, FunderRepoError> {
        sqlx::query!(
            "
            SELECT  
                fe.funder as name,
                fe.donor_status as kind,
                fe.company_number,
                 word_similarity($1, fe.funder) as similarity_score
            FROM funding_entries fe
            ORDER BY similarity_score DESC
            ",
            req.term(),
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?
        .into_iter()
        .filter(|r| r.similarity_score.unwrap_or(0_f32) >= req.strictness())
        .map(|r| {
            let kind = match &r.kind {
                Some(kind) => FunderKind::from_str(kind).unwrap(),
                None => FunderKind::NotSpecified,
            };
            let funder: Funder = match kind {
                FunderKind::Individual => IndividualFunder::new(r.name.clone()).into(),
                _ => CompanyFunder::new(r.name.clone(), r.company_number).into(),
            };
            let score = SearchSimilarity::from(r.similarity_score.unwrap_or(0_f32));
            Ok((funder, score))
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

        let term = EntitySearchRequest::new_strict("McDonald's".into()).unwrap();
        let results = db.get_funders_by_text_search_score(&term).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.first().unwrap().0.name(), "McDonald's");

        Ok(())
    }
}
