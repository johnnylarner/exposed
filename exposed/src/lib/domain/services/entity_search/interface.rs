use crate::domain::models::entity_search::{Entity, EntitySearchError, EntitySearchRequest};

/// Allows users to search the databse using free text.
pub trait EntitySearchService: Clone + Send + Sync + 'static {
    /// Returns entities based on free text search
    fn search_entities(
        &self,
        req: &EntitySearchRequest,
    ) -> impl Future<Output = Result<Vec<Entity>, EntitySearchError>> + Send;
}
