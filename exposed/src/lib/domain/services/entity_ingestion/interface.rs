use crate::domain::models::entity_ingestion::{
    EntityIngestionError, EntityIngestionOutcome, EntityIngestionRequest,
};

/// Runs one typed ingestion operation with only its required capabilities.
pub trait EntitySearchIngestionService: Clone + Send + Sync + 'static {
    /// Return a completed capture or committed load identity.
    fn run_ingestion(
        &self,
        req: &EntityIngestionRequest,
    ) -> impl Future<Output = Result<EntityIngestionOutcome, EntityIngestionError>> + Send;
}
