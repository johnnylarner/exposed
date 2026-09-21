use thiserror::Error;

/// Allows users to search the databse using free text.
pub trait FunderRepo: Clone + Send + Sync + 'static {
    /// Returns entities based on free text search
    fn get_funders_by_text_search(
        &self,
    ) -> impl Future<Output = Result<(), FunderRepoError>> + Send;
}

/// Errors that can occur when interacting with the repo
#[derive(Error, Debug)]
pub enum FunderRepoError {
    /// Generic error from the database
    #[error("unable to get search results due to {0}")]
    DatabaseError(String),
}
