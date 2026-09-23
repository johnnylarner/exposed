use axum::{Router, routing::get};

use crate::{
    domain::services::entity_search::EntitySearchService,
    inbound::http::{handlers, state::AppState},
};

/// Routes for the exposed application
pub fn routes<E: EntitySearchService>() -> Router<AppState<E>> {
    Router::new().route("/search", get(handlers::search_entities))
}
