#[path = "common/response_schema.rs"]
mod response_schema;
use schemars::JsonSchema;
use std::{
    io::{self, Write},
    process::ExitCode,
};

use exposed::inbound::cli::{CliSuccess, IntoResponse};
use serde::{Serialize, Serializer};

#[derive(Serialize, JsonSchema)]
struct Counts {
    records: usize,
}

#[derive(Serialize, JsonSchema)]
struct Imported {
    ingestion: Option<Counts>,
}

#[test]
fn nested_typed_payload_and_absence_are_json() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result: anyhow::Result<_> = Ok(CliSuccess::new(Imported {
        ingestion: Some(Counts { records: 3 }),
    }));
    let code = result
        .into_response()
        .into_response()
        .write_to(&mut stdout, &mut stderr)
        .unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(stdout, b"{\"ingestion\":{\"records\":3}}\n");
    assert!(stderr.is_empty());
    stdout.clear();
    CliSuccess::new(Imported { ingestion: None })
        .into_response()
        .write_to(&mut stdout, &mut stderr)
        .unwrap();
    assert_eq!(stdout, b"{\"ingestion\":null}\n");
}

#[test]
fn command_error_routes_full_context_to_stderr_and_fails() {
    let error = anyhow::Error::new(io::Error::new(io::ErrorKind::NotFound, "missing run"))
        .context("loading declarations");
    let result: anyhow::Result<CliSuccess<Counts>> = Err(error);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = result
        .into_response()
        .write_to(&mut stdout, &mut stderr)
        .unwrap();
    assert_eq!(code, ExitCode::FAILURE);
    assert!(stdout.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    response_schema::validate(&["util", "latest"], "Failure JSON schema:", &payload);
    assert_eq!(
        payload,
        serde_json::json!({"error": {"message": "loading declarations: missing run"}})
    );
    assert_eq!(stderr.last(), Some(&b'\n'));
}

#[test]
fn serialization_failure_cannot_emit_partial_success() {
    #[derive(JsonSchema)]
    struct BrokenPayload;
    impl Serialize for BrokenPayload {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::{Error, SerializeSeq};
            let mut sequence = serializer.serialize_seq(Some(2))?;
            sequence.serialize_element(&1)?;
            Err(S::Error::custom("payload failed"))
        }
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = CliSuccess::new(BrokenPayload)
        .into_response()
        .write_to(&mut stdout, &mut stderr)
        .unwrap();
    assert_eq!(code, ExitCode::FAILURE);
    assert!(stdout.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    response_schema::validate(&["util", "latest"], "Failure JSON schema:", &payload);
    assert_eq!(payload["error"]["message"], "payload failed");
}

struct BrokenWriter;
impl Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed output"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn success_and_error_writer_failures_are_returned() {
    let error = CliSuccess::new(Counts { records: 1 })
        .into_response()
        .write_to(BrokenWriter, Vec::new())
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    let result: Result<CliSuccess<Counts>, _> = Err(io::Error::other("command failed"));
    let error = result
        .into_response()
        .write_to(Vec::new(), BrokenWriter)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}

#[test]
fn canonical_ingestion_types_define_json_spellings() {
    use exposed::domain::models::{
        declaration_loading::DeclarationLoadOutcome,
        entity_ingestion::{DeclarationIngestionStage, IngestionKey, MemberIngestionStage},
    };
    let key = IngestionKey::default();
    assert_eq!(serde_json::to_value(&key).unwrap(), key.to_string());
    for (stage, expected) in [
        (DeclarationIngestionStage::Fetch, "fetch"),
        (DeclarationIngestionStage::Clean, "clean"),
        (DeclarationIngestionStage::Resolve, "resolve"),
        (DeclarationIngestionStage::Load, "load"),
    ] {
        assert_eq!(serde_json::to_value(stage).unwrap(), expected);
    }
    for (stage, expected) in [
        (MemberIngestionStage::Fetch, "fetch"),
        (MemberIngestionStage::Load, "load"),
    ] {
        assert_eq!(serde_json::to_value(stage).unwrap(), expected);
    }
    assert_eq!(
        serde_json::to_value(DeclarationLoadOutcome::Imported).unwrap(),
        "imported"
    );
    assert_eq!(
        serde_json::to_value(DeclarationLoadOutcome::AlreadyLoaded).unwrap(),
        "already_loaded"
    );
}

#[test]
fn invalid_database_connection_returns_json_instead_of_panicking() -> anyhow::Result<()> {
    let temporary = tempfile::TempDir::new()?;
    let config = temporary.path().join("config.yaml");
    std::fs::write(
        &config,
        rust_yaml::to_string(&exposed::inbound::cli::config::LoaderConfig {
            data_dir: temporary.path().join("data"),
        })?,
    )?;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_exposed"))
        .current_dir(temporary.path())
        .env("DATABASE_URL", "invalid-url")
        .args(["data", "members", "load", "--config"])
        .arg(&config)
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr)?;
    assert!(
        error["error"]["message"]
            .as_str()
            .is_some_and(|message| !message.is_empty())
    );
    Ok(())
}
