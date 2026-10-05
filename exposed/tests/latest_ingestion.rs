//! CLI coverage for the latest ingestion key and path.

use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::{Duration, SystemTime},
};

use exposed::{domain::models::entity_ingestion::IngestionKey, outbound::ExposedDataPipeline};
use tempfile::TempDir;
use uuid::Uuid;

fn run_latest(root: &Path) -> anyhow::Result<Output> {
    fs::write(root.join(".env"), "")?;
    fs::write(root.join("config.yaml"), "data_dir: ./data\n")?;
    Ok(Command::new(env!("CARGO_BIN_EXE_exposed"))
        .current_dir(root)
        .env_remove("DATABASE_URL")
        .args(["data", "latest", "config.yaml"])
        .output()?)
}

fn stdout(output: &Output) -> &str {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::str::from_utf8(&output.stdout).unwrap()
}

fn create_run(data: &Path, key: &IngestionKey, seconds: u64) -> anyhow::Result<()> {
    ExposedDataPipeline::new_with_ingestion_key(&data.to_path_buf(), key.clone())?;
    fs::File::open(data.join(key.to_string()))?
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))?;
    Ok(())
}

#[test]
fn reports_only_the_latest_key_and_absolute_run_path_without_a_database() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let data = tmp.path().join("data");
    let older: IngestionKey = "ffffffff-ffff-4fff-8fff-ffffffffffff".parse()?;
    let latest: IngestionKey = "00000000-0000-4000-8000-000000000001".parse()?;
    create_run(&data, &older, 10)?;
    create_run(&data, &latest, 20)?;
    fs::create_dir(data.join("not-an-ingestion-key"))?;
    fs::write(
        data.join(IngestionKey::default().to_string()),
        "not a directory",
    )?;

    let output = run_latest(tmp.path())?;
    let path = data.canonicalize()?.join(latest.to_string());

    assert_eq!(
        stdout(&output),
        format!("Ingestion key: {latest}\nPath: {}\n", path.display())
    );
    assert_eq!(fs::read_dir(&data)?.count(), 4);
    Ok(())
}

#[test]
fn equal_timestamps_use_the_greatest_uuid_regardless_of_creation_order() -> anyhow::Result<()> {
    let lower = IngestionKey::new(Uuid::from_u128(1));
    let higher = IngestionKey::new(Uuid::from_u128(2));
    for keys in [[&lower, &higher], [&higher, &lower]] {
        let tmp = TempDir::new()?;
        let data = tmp.path().join("data");
        for key in keys {
            create_run(&data, key, 10)?;
        }

        let output = run_latest(tmp.path())?;
        let path = data.canonicalize()?.join(higher.to_string());

        assert_eq!(
            stdout(&output),
            format!("Ingestion key: {higher}\nPath: {}\n", path.display())
        );
    }
    Ok(())
}

#[test]
fn missing_or_empty_store_has_no_runs_and_stays_unchanged() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let data = tmp.path().join("data");
    assert_eq!(
        stdout(&run_latest(tmp.path())?),
        "No ingestion runs found.\n"
    );
    assert!(!data.exists());

    fs::create_dir(&data)?;
    assert_eq!(
        stdout(&run_latest(tmp.path())?),
        "No ingestion runs found.\n"
    );
    assert_eq!(fs::read_dir(&data)?.count(), 0);
    Ok(())
}

#[test]
fn invalid_store_path_fails_with_a_read_error() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    fs::write(tmp.path().join("data"), "not a directory")?;

    let output = run_latest(tmp.path())?;

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unable to read pipeline data"));
    Ok(())
}
