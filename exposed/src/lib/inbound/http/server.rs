use std::sync::Arc;

use tokio::net::TcpListener;

use crate::{
    domain::services::entity_search::Service as EntitySearchService,
    inbound::http::{config::ServerConfig, routes::routes, state::AppState},
    outbound::ExposedDatabase,
};

/// Starts an `exposed` HTTP server
///
/// # Errors
/// - When the HTTP server fails
pub async fn serve_exposed(config: &ServerConfig, listener: TcpListener) -> anyhow::Result<()> {
    let db = ExposedDatabase::new(&config.connection_string).await;
    let service = EntitySearchService::new(db.clone(), db);

    let state = AppState {
        entity_search_service: Arc::new(service),
    };
    let router = routes().with_state(state);

    axum::serve(listener, router).await?;

    Ok(())
}
