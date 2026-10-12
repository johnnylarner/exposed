use thiserror::Error;

use crate::domain::models::entity_search::{
    EntitySearchRequest, FunderSearchMatch, FunderSearchResults, PhoneticRank,
};

/// Allows users to search the databse using free text.
pub trait FunderRepo: Clone + Send + Sync + 'static {
    /// Returns entities based on free text search
    fn get_funders_by_text_search_score(
        &self,
        term: &EntitySearchRequest,
    ) -> impl Future<Output = Result<FunderSearchResults, FunderRepoError>> + Send;
    /// Individual identities whose name words match the phonetic query.
    fn get_funders_by_phonetic_search(
        &self,
        req: &EntitySearchRequest,
    ) -> impl Future<Output = Result<Vec<(FunderSearchMatch, PhoneticRank)>, FunderRepoError>> + Send;
}

/// Errors that can occur when interacting with the repo
#[derive(Error, Debug)]
pub enum FunderRepoError {
    /// Generic error from the database
    #[error("unable to get search results due to {0}")]
    DatabaseError(String),
}
