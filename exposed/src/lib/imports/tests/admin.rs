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

#[sqlx::test(migrations = "../db/migrations")]
async fn cancelling_blocked_publication_rolls_back_current_member_and_keeps_completed_member(
    pool: PgPool,
) {
    use crate::imports::{
        adapters::parliament::{Parliament, Request},
        core::refresh::refresh_members,
    };
    use std::time::Duration;

    let store = PostgresStore::new(pool.clone());
    refresh_members(
        super::date("2024-07-04"),
        super::date("2026-09-15"),
        &Parliament(Fixture::new(vec![1, 2])),
        &store,
    )
    .await
    .unwrap();
    let state = AdminState::new(store, Arc::new(config()), None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let url = format!("http://{address}/imports/{}", uuid::Uuid::new_v4());
    let client = reqwest::Client::new();
    let mut message: Value = client
        .post(&url)
        .json(&json!({"kind":"declarations"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut fixture = Fixture::new(vec![1, 2]);
    fixture
        .declarations
        .insert(1, vec![super::refresh::declaration(101, 1, "10")]);
    fixture
        .declarations
        .insert(2, vec![super::refresh::declaration(102, 2, "20")]);
    let endpoint = format!("{url}/evidence");
    let blocked_body = loop {
        let request: Request = serde_json::from_value(message["request"].clone()).unwrap();
        let second_member = matches!(request, Request::Declarations { member: 2, .. });
        let response = fixture.fetch(request).await.unwrap();
        let body = json!({"sequence":message["sequence"],"response":response});
        if second_member {
            break body;
        }
        message = client
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    };
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE exposed.declarations IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let publication = tokio::spawn({
        let client = client.clone();
        async move { client.post(endpoint).json(&blocked_body).send().await }
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'INSERT INTO exposed.declarations%')",
            ).fetch_one(&pool).await.unwrap();
            if waiting { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("publication should block on the database lock");
    let cancellation = client
        .delete(&url)
        .timeout(Duration::from_secs(1))
        .send()
        .await;
    blocker.rollback().await.unwrap();
    let _ = publication.await.unwrap();
    server.abort();
    assert!(
        cancellation.is_ok(),
        "cancellation must not wait for publication: {cancellation:?}"
    );
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT source_declaration_id FROM exposed.declarations ORDER BY source_declaration_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(ids, vec![101]);
}
