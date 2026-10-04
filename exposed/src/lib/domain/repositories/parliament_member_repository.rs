use thiserror::Error;

use crate::domain::models::{
    declaration_ingestion::MemberAsId, entity_search::EntitySearchRequest,
    parliament_member::ParliamentMember, search_similarity::SearchSimilarity,
};

/// Allows users to search the databse using free text.
pub trait ParliamentMemberRepo: Clone + Send + Sync + 'static {
    /// All stored member identities, including former MPs, ordered by Parliament ID.
    fn get_stored_member_ids(
        &self,
    ) -> impl Future<Output = Result<Vec<MemberAsId>, ParliamentMemberRepoError>> + Send;

    /// Returns entities based on free text search
    fn get_members_by_text_search_score(
        &self,
        params: &EntitySearchRequest,
    ) -> impl Future<
        Output = Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError>,
    > + Send;

    /// Inserts members or updates existing members with matching Parliament IDs.
    fn upsert_members(
        &self,
        members: &[ParliamentMember],
    ) -> impl Future<Output = Result<(), ParliamentMemberRepoError>> + Send;
}

/// Errors that can occur when interacting with the repo
#[derive(Error, Debug)]
pub enum ParliamentMemberRepoError {
    /// Generic error from the database
    #[error("unable to get search results due to {0}")]
    DatabaseError(String),
}
