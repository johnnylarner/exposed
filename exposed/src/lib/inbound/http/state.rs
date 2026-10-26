use std::sync::Arc;

use crate::domain::services::{
    entity_details::EntityDetailsService, entity_search::EntitySearchService,
    rankings::RankingsService, updates::UpdatesService,
};

#[derive(Clone)]
/// State required to run the exposed app
pub struct AppState<
    E: EntitySearchService,
    D: EntityDetailsService,
    R: RankingsService,
    U: UpdatesService,
> {
    /// Current-month activity service
    pub updates_service: Arc<U>,
    /// Entity search service
    pub entity_search_service: Arc<E>,
    /// Current-term rankings service
    pub rankings_service: Arc<R>,
    /// Entity detail service
    pub entity_details_service: Arc<D>,
}
