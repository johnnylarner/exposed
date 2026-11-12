use std::process::Command;

use exposed::{
    domain::{
        models::{entity_ingestion::IngestionKey, lords::LordsCaptureBuilder},
        repositories::lords::LordsCaptureStorage,
    },
    outbound::LordsCaptureFiles,
};
use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgPool};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_exposed"))
}

#[test]
fn lords_schema_does_not_require_configuration_storage_network_or_database() {
    let directory = tempfile::tempdir().unwrap();
    for stage in ["fetch", "load"] {
        let output = binary()
            .args(["lords", stage, "--schema"])
            .current_dir(directory.path())
            .env_remove("DATABASE_URL")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let schema: Value = serde_json::from_slice(&output.stdout).unwrap();
        jsonschema::validator_for(&schema["success"]).unwrap();
        let errors = jsonschema::validator_for(&schema["failure"]).unwrap();
        assert!(errors.is_valid(&json!({"error":{"message":"source failed"}})));
        let output = binary().args(["lords", stage, "--help"]).output().unwrap();
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains(&format!("exposed lords {stage}")));
        assert!(help.contains("data_dir"));
        assert!(!help.contains("batch_size"));
    }
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn real_cli_lords_load_replays_a_complete_capture_and_reports_structured_failures(
    pool: PgPool,
) {
    let directory = tempfile::tempdir().unwrap();
    let key = IngestionKey::default();
    let storage = LordsCaptureFiles::new(directory.path(), key.clone()).unwrap();
    let mut builder =
        LordsCaptureBuilder::new(key.clone(), "2026-11-12T00:00:00Z".parse().unwrap());
    builder
        .push_page(include_bytes!("fixtures/lords-register-page.json").to_vec())
        .unwrap();
    storage.publish(&builder.finish().unwrap()).await.unwrap();
    let config = directory.path().join("config.yaml");
    std::fs::write(
        &config,
        format!("data_dir: {}\n", directory.path().display()),
    )
    .unwrap();
    let schema_output = binary()
        .args(["lords", "load", "--schema"])
        .output()
        .unwrap();
    let schema: Value = serde_json::from_slice(&schema_output.stdout).unwrap();
    let success = jsonschema::validator_for(&schema["success"]).unwrap();
    let failure = jsonschema::validator_for(&schema["failure"]).unwrap();
    for outcome in ["loaded", "already_loaded"] {
        let output = binary()
            .args(["lords", "load", "--config"])
            .arg(&config)
            .args(["--ingestion-key", &key.to_string()])
            .env(
                "DATABASE_URL",
                pool.connect_options().to_url_lossy().as_ref(),
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(success.is_valid(&response));
        assert_eq!(
            response,
            json!({"ingestion_key":key.to_string(), "outcome":outcome, "members":1})
        );
    }
    std::fs::remove_file(storage.path().join("manifest.json")).unwrap();
    let output = binary()
        .args(["lords", "load", "--config"])
        .arg(&config)
        .args(["--ingestion-key", &key.to_string()])
        .env(
            "DATABASE_URL",
            pool.connect_options().to_url_lossy().as_ref(),
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(failure.is_valid(&serde_json::from_slice(&output.stderr).unwrap()));
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_imports")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}
