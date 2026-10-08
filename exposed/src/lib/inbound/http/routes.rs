use axum::{Router, routing::get};

use crate::{
    domain::services::{entity_details::EntityDetailsService, entity_search::EntitySearchService},
    inbound::http::{handlers, state::AppState},
};

/// Routes for the exposed application
pub fn routes<E: EntitySearchService, D: EntityDetailsService>() -> Router<AppState<E, D>> {
    Router::new()
        .route("/search", get(handlers::search_entities))
        .route("/members/{id}", get(handlers::member_details))
        .route("/funders/{id}", get(handlers::funder_details))
}
