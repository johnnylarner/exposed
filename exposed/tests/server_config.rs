//! Server startup checks with isolated process environments.

use std::{
    process::{Child, Command},
    time::Duration,
};

use reqwest::StatusCode;
use sqlx::{ConnectOptions, PgPool};
use tempfile::TempDir;

fn server_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_exposed"));
    command.env_clear().arg("server");
    command
}

#[test]
fn requires_database_url_without_a_config_file() {
    let tmp = TempDir::new().unwrap();
    let output = server_command().current_dir(tmp.path()).output().unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("DATABASE_URL must be set"));
}

#[test]
fn rejects_invalid_database_urls_before_connecting() {
    for value in ["", "not-a-url"] {
        let output = server_command()
            .env("DATABASE_URL", value)
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("DATABASE_URL must be a valid"));
    }
}

#[test]
fn rejects_invalid_ports_before_connecting() {
    for value in ["", "abc", "-1", "65536"] {
        let output = server_command()
            .env("DATABASE_URL", "postgresql://localhost/exposed")
            .env("PORT", value)
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("PORT must be an integer between 0 and 65535")
        );
    }
}

#[test]
fn help_does_not_require_database_configuration() {
    let output = server_command().arg("--help").output().unwrap();

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("exposed server"));
    assert!(!help.contains("<CONFIG>"));
}

struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[sqlx::test(migrations = "../db/migrations")]
async fn serves_search_using_environment_configuration(pool: PgPool) {
    let tmp = TempDir::new().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let mut server = ServerProcess(
        server_command()
            .current_dir(tmp.path())
            .env(
                "DATABASE_URL",
                pool.connect_options().to_url_lossy().as_str(),
            )
            .env("PORT", port.to_string())
            .spawn()
            .unwrap(),
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let url = format!("http://127.0.0.1:{port}/search?term=example&max_entries=10");

    for _ in 0..50 {
        assert!(server.0.try_wait().unwrap().is_none(), "server exited");
        if let Ok(response) = client.get(&url).send().await {
            assert_eq!(response.status(), StatusCode::OK);
            let body: serde_json::Value =
                serde_json::from_slice(&response.bytes().await.unwrap()).unwrap();
            assert_eq!(body["entities"], serde_json::json!([]));
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    panic!("server did not accept requests");
}
