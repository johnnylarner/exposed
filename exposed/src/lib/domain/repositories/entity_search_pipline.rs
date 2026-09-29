use thiserror::Error;

/// Set of methods for operating on data in the entity
/// search pipeline
pub trait EntitySearchPipeline: Clone + Send + Sync + 'static {
    /// Reads raw data persisted from the parliament API    
    fn read_raw_data(&self) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
    /// Reads cleaned data
    fn read_cleaned_data(
        &self,
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
    /// Reads resolved data
    fn read_resolved_data(
        &self,
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
    /// Writes raw data pulled from the parliament API
    fn write_raw_data(&self) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
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
    /// Error when reading pipeline results
    #[error("unable to read pipeline data due to {0}")]
    ReadError(String),
    /// Error when writing pipeline results
    #[error("unable to write pipeline data due to {0}")]
    WriteError(String),
}
