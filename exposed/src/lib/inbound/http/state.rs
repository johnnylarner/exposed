use std::sync::Arc;

use crate::domain::services::entity_search::EntitySearchService;

#[derive(Clone)]
/// State required to run the exposed app
pub struct AppState<E: EntitySearchService> {
    /// Entity search service
    pub entity_search_service: Arc<E>,
}
