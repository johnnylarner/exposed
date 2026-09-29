use thiserror::Error;

/// Allows users to interact with the UK Parliament API.
pub trait ParliamentApi: Clone + Send + Sync + 'static {
    /// Returns members sitting in the current parliament
    fn get_sitting_members(&self) -> impl Future<Output = Result<(), ParliamentApiError>> + Send;
    /// Returns declarations for sitting members
    fn get_declarations_for_sitting_members(
        &self,
    ) -> impl Future<Output = Result<(), ParliamentApiError>> + Send;
}

/// Errors that can occur when interacting with the repo
#[derive(Error, Debug)]
pub enum ParliamentApiError {
    /// Generic error from the database
    #[error("unable to get API results due to {0}")]
    ApiError(String),
}
