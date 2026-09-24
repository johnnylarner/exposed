use std::sync::Arc;

use crate::{
    config::Config,
    domain::services::entity_search::Service as EntitySearchService,
    inbound::http::{routes::routes, state::AppState},
    outbound::ExposedDatabase,
};

/// Starts an `exposed` HTTP server
pub async fn serve_exposed(config: &Config) -> anyhow::Result<()> {
    let db = ExposedDatabase::new(&config.connection_string).await;
    let service = EntitySearchService::new(db.clone(), db);

    let state = AppState {
        entity_search_service: Arc::new(service),
    };
    let router = routes().with_state(state);

    let port = config.port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    let _ = axum::serve(listener, router).await?;

    Ok(())
}
