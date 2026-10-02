use crate::domain::models::entity_ingestion::{
    EntityIngestionError, EntityIngestionRequest, IngestionKey,
};

/// Retrieves raw data
pub trait EntityFetcherService: Clone + 'static {
    /// Fetches members from the UK Parliament API
    fn fetch_members(
        &self,
        req: &EntityIngestionRequest,
    ) -> impl Future<Output = Result<IngestionKey, EntityIngestionError>> + Send;
}
