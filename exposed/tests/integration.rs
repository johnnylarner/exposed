//! Search remains available alongside the import runtime, using an isolated database.
use exposed::{
    domain::services::entity_search::Service,
    inbound::http::{routes::routes, state::AppState},
    outbound::ExposedDatabase,
};
use sqlx::PgPool;
use std::sync::Arc;

#[sqlx::test(migrations = "../db/migrations")]
async fn search_returns_one_shared_funder(pool: PgPool) {
    sqlx::query("INSERT INTO exposed.funders (funder_name, funder_kind) VALUES ('hsbc uk bank plc','Company')").execute(&pool).await.unwrap();
    let database = ExposedDatabase::from(pool);
    let state = AppState {
        entity_search_service: Arc::new(Service::new(database.clone(), database)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, routes().with_state(state))
            .await
            .unwrap();
    });
    let response = reqwest::get(format!(
        "http://{address}/search?term=hsbc&strictness=0.9&max_entries=10"
    ))
    .await
    .unwrap();
    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["entities"].as_array().unwrap().len(), 1);
    assert_eq!(body["entities"][0]["name"], "hsbc uk bank plc");
    task.abort();
}
