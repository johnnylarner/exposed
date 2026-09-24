use std::sync::Arc;

use exposed::{
    domain::services::entity_search::Service,
    inbound::http::{routes::routes, state::AppState},
    outbound::ExposedDatabase,
};
use sqlx::{ConnectOptions, PgPool};
use tokio::{net::TcpListener, task::JoinHandle};

pub struct TestApp {
    pub url: String,
    task: JoinHandle<()>,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn start_app(pool: &PgPool) -> anyhow::Result<TestApp> {
    let db = ExposedDatabase::new(pool.connect_options().to_url_lossy().as_str()).await;
    let router = routes().with_state(AppState {
        entity_search_service: Arc::new(Service::new(db.clone(), db)),
    });
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    Ok(TestApp { url, task })
}
