use crate::domain::{models::entity::Entity, services::entity_search::error::EntitySearchError};

/// Allows users to search the databse using free text.
pub trait EntitySearchService<P, F>: Clone + Send + Sync + 'static {
    /// Returns entities based on free text search
    fn search_entities(
        &self,
    ) -> impl Future<Output = Result<Vec<Entity>, EntitySearchError>> + Send;
}
