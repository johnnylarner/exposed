use crate::domain::models::entity_ingestion::{EntityIngestionError, EntityIngestionRequest};

/// Prepares UK Parliament API data for storage
pub trait EntitySearchIngestionService: Clone + Send + Sync + 'static {
    /// Runs entity ingestion
    fn run_ingestion(
        &self,
        req: &EntityIngestionRequest,
    ) -> impl Future<Output = Result<(), EntityIngestionError>> + Send;
}
