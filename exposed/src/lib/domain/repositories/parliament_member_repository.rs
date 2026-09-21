use thiserror::Error;

use crate::domain::models::{
    parliament_member::ParliamentMember, search_similarity::SearchSimilarity,
};

/// Allows users to search the databse using free text.
pub trait ParliamentMemberRepo: Clone + Send + Sync + 'static {
    /// Returns entities based on free text search
    fn get_members_by_text_search_score(
        &self,
    ) -> impl Future<
        Output = Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError>,
    > + Send;
}

/// Errors that can occur when interacting with the repo
#[derive(Error, Debug)]
pub enum ParliamentMemberRepoError {
    /// Generic error from the database
    #[error("unable to get search results due to {0}")]
    DatabaseError(String),
}
