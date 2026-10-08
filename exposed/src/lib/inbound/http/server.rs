use std::sync::Arc;

use tokio::net::TcpListener;

use crate::{
    domain::services::entity_search::Service as EntitySearchService,
    inbound::http::{config::ServerConfig, routes::routes, state::AppState},
    outbound::ExposedDatabase,
};

/// Loads server configuration and serves HTTP on port 6999 or an available port.
///
/// # Errors
/// Returns an error if configuration, runtime creation, or server startup fails.
pub fn run_server() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = ServerConfig::from_env()?;

    tokio::runtime::Runtime::new()?.block_on(async {
        let listener = match TcpListener::bind("0.0.0.0:6999").await {
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                TcpListener::bind("0.0.0.0:0").await?
            }
            result => result?,
        };
        println!(
            "Listening on http://localhost:{}",
            listener.local_addr()?.port()
        );
        serve_exposed(&config, listener).await
    })
}

/// Starts an `exposed` HTTP server
///
/// # Errors
/// - When the HTTP server fails
pub async fn serve_exposed(config: &ServerConfig, listener: TcpListener) -> anyhow::Result<()> {
    let db = ExposedDatabase::new(&config.connection_string).await;
    let service = EntitySearchService::new(db.clone(), db.clone());

    let state = AppState {
        entity_search_service: Arc::new(service),
        entity_details_service: Arc::new(crate::domain::services::entity_details::Service::new(db)),
    };
    let router = routes().with_state(state);

    axum::serve(listener, router).await?;

    Ok(())
}
