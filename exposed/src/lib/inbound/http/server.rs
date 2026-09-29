use crate::{
    domain::services::entity_search::EntitySearchService,
    inbound::http::{routes::routes, state::AppState},
};
use tokio::net::TcpListener;

/// Starts an `exposed` HTTP server
pub async fn serve_exposed<E: EntitySearchService>(
    listener: TcpListener,
    state: AppState<E>,
) -> anyhow::Result<()> {
    let router = routes().with_state(state);

    axum::serve(listener, router).await?;

    Ok(())
}
