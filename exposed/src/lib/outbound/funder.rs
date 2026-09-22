//! Concrete implementation of the repository

use std::str::FromStr;

use crate::{
    domain::{
        models::{
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
    ) -> Result<
        Vec<(Funder, SearchSimilarity)>,
        crate::domain::repositories::funder_repository::FunderRepoError,
    > {
        let word = "McDonald";
        sqlx::query!(
            "
            SELECT  
                fe.funder as name,
                fe.donor_status as kind,
                fe.company_number,
                 word_similarity($1, fe.funder) as similarity_score
            FROM funding_entries fe
            ORDER BY 3 DESC
            ",
            word,
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| FunderRepoError::DatabaseError(e.to_string()))?
        .into_iter()
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
