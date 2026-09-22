//! Concrete implementation of the repository

use std::str::FromStr;

use crate::{
    domain::{
        models::{parliament_member::ParliamentMember, search_similarity::SearchSimilarity},
        repositories::parliament_member_repository::{
            ParliamentMemberRepo, ParliamentMemberRepoError,
        },
    },
    outbound::postgres::ExposedDatabase,
};

impl ParliamentMemberRepo for ExposedDatabase {
    async fn get_members_by_text_search_score(
        &self,
    ) -> Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError> {
        let word = "McDonald";
        sqlx::query!(
            "
            SELECT  
                m.name,
                m.parliament_member_id,
                m.party_name,
                m.latest_membership_from as constituency,
                 word_similarity($1, m.name) as similarity_score
            FROM members m
            ORDER BY 3 DESC
            ",
            word,
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ParliamentMemberRepoError::DatabaseError(e.to_string()))?
        .into_iter()
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
