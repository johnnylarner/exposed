use thiserror::Error;

use crate::domain::models::{
    declaration_ingestion::CapturedDeclaration,
    entity_ingestion::IngestionKey,
    parliament_member::{MemberId, ParliamentMember},
};

/// Set of I/O methods for data in the entity search pipeline
pub trait EntityIngestionStorage: Clone + Send + Sync + 'static {
    /// Writes and closes one member's declaration file, including an empty result.
    /// Each record contains the declaration and its nested funding entries.
    fn write_raw_declarations(
        &self,
        member: MemberId,
        declarations: &[CapturedDeclaration],
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;

    /// Key used for storage
    fn ingestion_key(&self) -> IngestionKey;
    /// Reads raw data persisted from the parliament API    
    fn read_raw_members(
        &self,
    ) -> impl Future<Output = Result<Vec<ParliamentMember>, EntitySearchPipelineError>> + Send;
    /// Reads cleaned data
    fn read_cleaned_data(
        &self,
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
    /// Reads resolved data
    fn read_resolved_data(
        &self,
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
    /// Writes raw data pulled from the parliament API
    fn write_raw_members(
        &self,
        members: &[ParliamentMember],
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
    /// Writes cleaned data pulled from the parliament API
    fn write_cleaned_data(
        &self,
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
    /// Writes resolved data pulled from the parliament API
    fn write_resolved_data(
        &self,
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
}

/// Errors that can occur when interacting with the repo
#[derive(Error, Debug)]
pub enum EntitySearchPipelineError {
    /// Generic error when something unexpected goes wrong
    #[error("unexpected error occured: {0}")]
    Unexpected(String),
    /// Error when reading pipeline results
    #[error("unable to read pipeline data due to {0}")]
    ReadError(String),
    /// Error when writing pipeline results
    #[error("unable to write pipeline data due to {0}")]
    WriteError(String),
}
