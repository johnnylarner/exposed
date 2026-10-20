//! Launcher checks run in isolated processes so global subscribers and dotenv
//! never change the integration-test process's environment.

use std::process::{Command, Output};

fn server(directory: &std::path::Path, rust_log: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_exposed"));
    command
        .current_dir(directory)
        .arg("server")
        .env_remove("DATABASE_URL")
        .env_remove("RUST_LOG");
    if let Some(filter) = rust_log {
        command.env("RUST_LOG", filter);
    }
    command.output().unwrap()
}

#[test]
fn invalid_explicit_filter_fails_once_before_server_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let output = server(directory.path(), Some("exposed["));
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(
        stderr.contains("server tracing initialization failed"),
        "{stderr}"
    );
    assert!(stderr.contains("invalid"), "{stderr}");
    assert!(!stderr.contains("DATABASE_URL"));
}

#[test]
fn missing_filter_uses_default_and_startup_error_is_structured_once_on_stderr() {
    let directory = tempfile::tempdir().unwrap();
    let output = server(directory.path(), None);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    let events: Vec<serde_json::Value> = stderr
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["level"], "ERROR");
    assert_eq!(events[0]["fields"]["name"], "server.failed");
    assert!(
        events[0]["fields"]["error.message"]
            .as_str()
            .unwrap()
            .contains("DATABASE_URL must be set")
    );
}

#[test]
fn dotenv_filter_is_loaded_before_subscriber_initialization() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join(".env"), "RUST_LOG=exposed[\n").unwrap();
    let output = server(directory.path(), None);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("server tracing initialization failed"),
        "{stderr}"
    );
    assert!(!stderr.contains("DATABASE_URL"));
}
