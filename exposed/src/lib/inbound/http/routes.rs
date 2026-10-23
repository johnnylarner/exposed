use axum::{Router, http::StatusCode, routing::get};

use crate::{
    domain::services::{entity_details::EntityDetailsService, entity_search::EntitySearchService},
    inbound::http::{handlers, state::AppState},
};

/// Routes for the exposed application
pub fn routes<E: EntitySearchService, D: EntityDetailsService>() -> Router<AppState<E, D>> {
    let router = Router::new()
        .route("/healthz", get(|| async { StatusCode::NO_CONTENT }))
        .route("/overview", get(handlers::overview))
        .route("/search", get(handlers::search_entities))
        .route("/members/{id}", get(handlers::member_details))
        .route("/funders/{id}", get(handlers::funder_details));
    super::tracing::instrument(router)
}
