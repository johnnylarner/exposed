use anyhow::Context;
use std::sync::Arc;

use tokio::net::TcpListener;

use crate::{
    domain::services::entity_search::Service as EntitySearchService,
    inbound::http::{config::ServerConfig, routes::routes, state::AppState},
    outbound::ExposedDatabase,
};

/// Subscriber installation failed before server startup could use tracing.
#[derive(Debug, thiserror::Error)]
pub enum SubscriberInitializationError {
    /// `RUST_LOG` contains a non-Unicode value.
    #[error("server tracing initialization failed: invalid RUST_LOG")]
    Environment(#[from] std::env::VarError),
    /// `RUST_LOG` contains invalid filter directives.
    #[error("server tracing initialization failed: invalid filter")]
    Filter(#[from] tracing_subscriber::filter::ParseError),
    /// A process-wide subscriber could not be installed.
    #[error("server tracing initialization failed: subscriber installation failed")]
    Install(#[from] tracing_subscriber::util::TryInitError),
}

/// Loads server configuration and serves HTTP on port 6999.
///
/// Initializes a process-wide subscriber after loading dotenv. Embedded hosts
/// with their own subscriber should call [`serve_exposed`] instead.
///
/// # Errors
/// Returns an error if subscriber setup, configuration, runtime creation, or
/// server startup fails.
pub fn run_server() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    super::tracing::init_subscriber()?;
    let config = ServerConfig::from_env().context("reading HTTP server configuration")?;

    tokio::runtime::Runtime::new()
        .context("creating HTTP server runtime")?
        .block_on(async {
            let listener = TcpListener::bind("0.0.0.0:6999")
                .await
                .context("binding HTTP server port 6999")?;
            serve_exposed(&config, listener).await
        })
}

/// Starts an `exposed` HTTP server
///
/// # Errors
/// Returns an error if connecting to the database or serving HTTP fails.
pub async fn serve_exposed(config: &ServerConfig, listener: TcpListener) -> anyhow::Result<()> {
    let db = ExposedDatabase::new(&config.connection_string)
        .await
        .context("connecting HTTP server database")?;
    let service = EntitySearchService::new(db.clone(), db.clone());

    let state = AppState {
        entity_search_service: Arc::new(service),
        entity_details_service: Arc::new(crate::domain::services::entity_details::Service::new(db)),
    };
    let router = routes().with_state(state);

    tracing::info!(address = %listener.local_addr()?, "HTTP server listening");
    axum::serve(listener, router)
        .await
        .context("serving HTTP requests")?;

    Ok(())
}
