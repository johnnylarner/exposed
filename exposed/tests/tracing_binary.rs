use std::{
    fs,
    process::{Command, Output},
};

fn command(directory: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_exposed"));
    command
        .current_dir(directory)
        .env_remove("EXPOSED_LOG_FILE")
        .env_remove("RUST_LOG")
        .env_remove("DATABASE_URL");
    command
}

fn events(path: &std::path::Path) -> Vec<serde_json::Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn errors(path: &std::path::Path) -> Vec<serde_json::Value> {
    events(path)
        .into_iter()
        .filter(|event| event["level"] == "ERROR")
        .collect()
}

fn failure_json(output: &Output) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(payload["error"]["message"].is_string());
}

#[test]
fn cli_file_sink_preserves_streams_and_appends_one_event_per_failure() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("trace.jsonl");
    let args = ["data", "util", "latest", "--config", "missing-config.yaml"];
    let plain = command(temp.path())
        .args(args)
        .env("RUST_LOG", "trace")
        .output()
        .unwrap();
    failure_json(&plain);
    for expected in [1, 2] {
        let logged = command(temp.path())
            .args(args)
            .env("EXPOSED_LOG_FILE", &path)
            .output()
            .unwrap();
        assert_eq!(logged.status.code(), plain.status.code());
        assert_eq!(logged.stdout, plain.stdout);
        assert_eq!(logged.stderr, plain.stderr);
        let errors = errors(&path);
        assert_eq!(errors.len(), expected);
        assert_eq!(errors.last().unwrap()["span"]["command"], "data");
        assert_eq!(errors.last().unwrap()["span"]["utility"], "latest");
    }
    let output = command(temp.path())
        .args(["data", "util", "uuid-v7"])
        .env("EXPOSED_LOG_FILE", &path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["uuid"].is_string()
    );
    assert_eq!(errors(&path).len(), 2);
    assert!(
        events(&path)
            .iter()
            .any(|event| event["fields"]["message"] == "command completed")
    );
}

#[test]
fn schema_and_help_bypass_sink_setup_and_invalid_sinks_preserve_command_protocols() {
    let temp = tempfile::tempdir().unwrap();
    let invalid = temp.path().join("absent-parent/trace.jsonl");
    for args in [vec!["data", "util", "uuid-v7", "--schema"], vec!["--help"]] {
        let plain = command(temp.path()).args(&args).output().unwrap();
        let logged = command(temp.path())
            .args(&args)
            .env("EXPOSED_LOG_FILE", &invalid)
            .output()
            .unwrap();
        assert!(logged.status.success());
        assert_eq!(logged.stdout, plain.stdout);
        assert_eq!(logged.stderr, plain.stderr);
        assert!(!invalid.exists());
    }
    for args in [vec!["data", "util", "uuid-v7"], vec!["migrate", "run"]] {
        let output = command(temp.path())
            .args(args)
            .env("EXPOSED_LOG_FILE", &invalid)
            .output()
            .unwrap();
        failure_json(&output);
        assert!(String::from_utf8_lossy(&output.stderr).contains("opening EXPOSED_LOG_FILE"));
    }
    for arg in ["server", "healthcheck"] {
        let output = command(temp.path())
            .arg(arg)
            .env("EXPOSED_LOG_FILE", &invalid)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("opening EXPOSED_LOG_FILE"));
    }
}

#[test]
fn server_startup_and_migration_failures_reach_their_own_boundaries_once() {
    let temp = tempfile::tempdir().unwrap();
    for args in [vec!["server"], vec!["migrate", "run"]] {
        let path = temp.path().join(format!("{}.jsonl", args[0]));
        let output = command(temp.path())
            .args(&args)
            .env("EXPOSED_LOG_FILE", &path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let errors = errors(&path);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]["fields"]["error"]
                .as_str()
                .unwrap()
                .contains("DATABASE_URL")
        );
        if args[0] == "migrate" {
            failure_json(&output);
        } else {
            assert!(String::from_utf8_lossy(&output.stderr).starts_with("Error:"));
        }
    }
}

#[test]
fn healthcheck_failure_is_logged_once_without_changing_its_diagnostic() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:6999").unwrap();
    let server = std::thread::spawn(move || {
        for status in [
            "503 Service Unavailable",
            "503 Service Unavailable",
            "204 No Content",
        ] {
            let (mut connection, _) = listener.accept().unwrap();
            connection
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = [0; 2048];
            assert!(connection.read(&mut bytes).unwrap() > 0);
            write!(
                connection,
                "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("healthcheck.jsonl");
    let output = command(temp.path())
        .arg("healthcheck")
        .env("EXPOSED_LOG_FILE", &path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected healthcheck HTTP 204"));
    let error_events = errors(&path);
    assert_eq!(error_events.len(), 1);
    assert_eq!(error_events[0]["span"]["command"], "healthcheck");
    fs::write(
        temp.path().join(".env"),
        "EXPOSED_LOG_FILE=absent-parent/trace.jsonl\n",
    )
    .unwrap();
    let without_sink = command(temp.path()).arg("healthcheck").output().unwrap();
    assert_eq!(without_sink.status.code(), output.status.code());
    assert_eq!(without_sink.stdout, output.stdout);
    assert_eq!(without_sink.stderr, output.stderr);
    let success = command(temp.path())
        .arg("healthcheck")
        .env("EXPOSED_LOG_FILE", &path)
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(success.status.success());
    assert!(success.stdout.is_empty());
    assert!(success.stderr.is_empty());
    assert_eq!(errors(&path).len(), 1);
    assert!(
        events(&path)
            .iter()
            .any(|event| event["fields"]["message"] == "command completed")
    );
}

#[test]
fn canonical_stage_context_and_filtering_apply_to_cli_events() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("stage.jsonl");
    let args = [
        "data",
        "declarations",
        "clean",
        "--config",
        "missing-config.yaml",
        "--ingestion-key",
        "019b733c-2360-7000-8000-000000000001",
    ];
    let output = command(temp.path())
        .args(args)
        .env("EXPOSED_LOG_FILE", &path)
        .output()
        .unwrap();
    failure_json(&output);
    let error_events = errors(&path);
    assert_eq!(error_events.len(), 1);
    assert_eq!(error_events[0]["span"]["family"], "declarations");
    assert_eq!(error_events[0]["span"]["stage"], "clean");
    let error_only = temp.path().join("error-only.jsonl");
    let error_output = command(temp.path())
        .args(args)
        .env("EXPOSED_LOG_FILE", &error_only)
        .env("RUST_LOG", "error")
        .output()
        .unwrap();
    assert_eq!(error_output.status.code(), output.status.code());
    assert_eq!(error_output.stdout, output.stdout);
    assert_eq!(error_output.stderr, output.stderr);
    let error_events = errors(&error_only);
    assert_eq!(error_events.len(), 1);
    assert_eq!(error_events[0]["span"]["command"], "data");
    assert_eq!(error_events[0]["span"]["family"], "declarations");
    assert_eq!(error_events[0]["span"]["stage"], "clean");
    let filtered = temp.path().join("filtered.jsonl");
    let muted = command(temp.path())
        .args(args)
        .env("EXPOSED_LOG_FILE", &filtered)
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert_eq!(muted.status.code(), output.status.code());
    assert_eq!(muted.stdout, output.stdout);
    assert_eq!(muted.stderr, output.stderr);
    assert!(events(&filtered).is_empty());
}
