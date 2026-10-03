use thiserror::Error;

use crate::domain::models::{
    declaration_ingestion::{CapturedDeclaration, StoredMember},
    parliament_member::ParliamentMember,
};

/// Allows users to interact with the UK Parliament API.
pub trait ParliamentApi: Clone + Send + Sync + 'static {
    /// Returns members sitting in the current parliament
    fn get_sitting_members(
        &self,
    ) -> impl Future<Output = Result<Vec<ParliamentMember>, ParliamentApiError>> + Send;
    /// All available Commons declarations for this stored member, including expired records.
    /// Includes every returned version and returns children as separate declarations.
    fn get_declarations(
        &self,
        member: StoredMember,
    ) -> impl Future<Output = Result<Vec<CapturedDeclaration>, ParliamentApiError>> + Send;
}

/// Errors that can occur when interacting with the repo
#[derive(Error, Debug)]
pub enum ParliamentApiError {
    /// Generic error from the database
    #[error("unable to get API results due to {0}")]
    ApiError(String),
}
