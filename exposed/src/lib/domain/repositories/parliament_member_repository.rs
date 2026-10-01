use thiserror::Error;

use crate::domain::models::{
    entity_search::EntitySearchRequest, member_ingestion::MemberRefresh,
    parliament_member::ParliamentMember, search_similarity::SearchSimilarity,
};

/// Search and atomically refresh Parliament members.
pub trait ParliamentMemberRepo: Clone + Send + Sync + 'static {
    /// Reconcile an accepted cohort in one transaction, preserving existing identities.
    fn refresh_members(
        &self,
        refresh: &MemberRefresh,
    ) -> impl Future<Output = Result<(), ParliamentMemberRepoError>> + Send;
    /// Returns entities based on free text search
    fn get_members_by_text_search_score(
        &self,
        params: &EntitySearchRequest,
    ) -> impl Future<
        Output = Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError>,
    > + Send;
}

/// Errors that can occur when interacting with the repo
#[derive(Error, Debug)]
pub enum ParliamentMemberRepoError {
    /// Generic error from the database
    #[error("member database operation failed: {0}")]
    DatabaseError(String),
}
