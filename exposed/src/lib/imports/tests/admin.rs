use super::refresh::Fixture;
use crate::imports::{
    adapters::{parliament::Transport, postgres::PostgresStore},
    admin::{AdminState, router},
    config::ImportConfig,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;

fn config() -> ImportConfig {
    serde_json::from_value(json!({"term_start":"2024-07-04","python":"unused","source_directory":"unused","admin_bind":"127.0.0.1:0","automatic_refresh":false})).unwrap()
}

#[sqlx::test(migrations = "../db/migrations")]
async fn operator_session_replays_ack_without_repeating_writes(pool: PgPool) {
    let state = AdminState::new(
        PostgresStore::new(pool.clone()),
        Arc::new(config()),
        Some("test-token".into()),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let url = format!("http://{address}/imports/{}", uuid::Uuid::new_v4());
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .post(&url)
            .json(&json!({"kind":"members"}))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let mut message: Value = client
        .post(&url)
        .bearer_auth("test-token")
        .json(&json!({"kind":"members"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let duplicate_begin: Value = client
        .post(&url)
        .bearer_auth("test-token")
        .json(&json!({"kind":"members"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(message, duplicate_begin);
    let fixture = Fixture::new(vec![1, 2]);
    while message.get("request").is_some() {
        let request = serde_json::from_value(message["request"].clone()).unwrap();
        let response = fixture.fetch(request).await.unwrap();
        let body = json!({"sequence":message["sequence"],"response":response});
        let endpoint = format!("{url}/evidence");
        message = client
            .post(&endpoint)
            .bearer_auth("test-token")
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let replay: Value = client
            .post(&endpoint)
            .bearer_auth("test-token")
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(message, replay);
    }
    assert_eq!(message["result"]["members"], 2);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM exposed.members")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
    task.abort();
}

#[sqlx::test(migrations = "../db/migrations")]
async fn python_operator_drives_real_rust_initialization_and_safe_repeat(pool: PgPool) {
    let state = AdminState::new(PostgresStore::new(pool.clone()), Arc::new(config()), None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let output = tokio::process::Command::new(root.join("ingest/.venv/bin/python"))
        .arg(root.join("ingest/tests/fixtures/operator_smoke.py"))
        .arg(format!("http://{address}"))
        .env("PYTHONPATH", root.join("ingest/src"))
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["members"]["unchanged"], 1);
    assert_eq!(summary["declarations"]["declarations"], 1);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM exposed.declarations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    task.abort();
}
