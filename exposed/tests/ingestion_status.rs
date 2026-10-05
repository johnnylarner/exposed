//! CLI coverage for read-only ingestion status.

use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::{Duration, SystemTime},
};

use exposed::{
    domain::{
        models::{declaration_ingestion::MemberAsId, entity_ingestion::IngestionKey},
        repositories::entity_ingestion::EntityIngestionStorage,
    },
    outbound::ExposedDataPipeline,
};
use tempfile::TempDir;
use uuid::Uuid;

fn run_latest(root: &Path) -> anyhow::Result<Output> {
    fs::write(root.join(".env"), "")?;
    // Existing fetch configuration works. Only data_dir is required.
    fs::write(
        root.join("config.yaml"),
        "data_dir: ./data\nbatch_size: 100\n",
    )?;
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

fn set_modified(path: &Path, seconds: u64) -> anyhow::Result<()> {
    fs::File::open(path)?.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))?;
    Ok(())
}

#[tokio::test]
async fn reports_both_datasets_written_by_the_pipeline_without_a_database() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let key = IngestionKey::default();
    let pipeline =
        ExposedDataPipeline::new_with_ingestion_key(&tmp.path().join("data"), key.clone())?;
    pipeline.write_raw_members(&[]).await?;
    pipeline
        .write_raw_declarations(MemberAsId::new(Uuid::from_u128(42), 512)?, &[])
        .await?;

    let output = run_latest(tmp.path())?;

    assert_eq!(
        stdout(&output),
        format!("Ingestion key: {key}\nDATASET\tSTAGE\nmembers\traw\ndeclarations\traw\n")
    );
    assert_eq!(fs::read_dir(tmp.path().join("data"))?.count(), 1);
    Ok(())
}

#[tokio::test]
async fn uses_nested_file_timestamps_and_does_not_mix_separate_runs() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    // UUIDv4 keys need not sort in the same order as dataset timestamps.
    let older: IngestionKey = "ffffffff-ffff-4fff-8fff-ffffffffffff".parse()?;
    let latest: IngestionKey = "00000000-0000-4000-8000-000000000001".parse()?;
    let data = tmp.path().join("data");
    let members = ExposedDataPipeline::new_with_ingestion_key(&data, older.clone())?;
    members.write_raw_members(&[]).await?;
    set_modified(
        &data.join(older.to_string()).join("raw/members.parquet"),
        10,
    )?;
    let declarations = ExposedDataPipeline::new_with_ingestion_key(&data, latest.clone())?;
    let member = MemberAsId::new(Uuid::from_u128(42), 512)?;
    declarations.write_raw_declarations(member, &[]).await?;
    set_modified(
        &data
            .join(latest.to_string())
            .join(format!("raw/declarations/{}.parquet", member.member_id())),
        20,
    )?;
    // Directory timestamps do not determine the latest dataset.
    set_modified(&data.join(latest.to_string()), 1)?;
    fs::create_dir_all(
        data.join(IngestionKey::default().to_string())
            .join("raw/declarations"),
    )?;

    let output = run_latest(tmp.path())?;

    assert_eq!(
        stdout(&output),
        format!("Ingestion key: {latest}\nDATASET\tSTAGE\ndeclarations\traw\n")
    );
    Ok(())
}

#[tokio::test]
async fn reports_the_furthest_stored_stage_for_each_dataset() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let key = IngestionKey::default();
    let data = tmp.path().join("data");
    let pipeline = ExposedDataPipeline::new_with_ingestion_key(&data, key.clone())?;
    pipeline.write_raw_members(&[]).await?;
    let member = MemberAsId::new(Uuid::from_u128(42), 512)?;
    pipeline.write_raw_declarations(member, &[]).await?;
    let run = data.join(key.to_string());
    fs::create_dir_all(run.join("resolved"))?;
    fs::copy(
        run.join("raw/members.parquet"),
        run.join("resolved/members.parquet"),
    )?;
    fs::create_dir_all(run.join("cleaned/declarations"))?;
    let partition = format!("declarations/{}.parquet", member.member_id());
    fs::copy(
        run.join("raw").join(&partition),
        run.join("cleaned").join(&partition),
    )?;

    let output = run_latest(tmp.path())?;

    assert_eq!(
        stdout(&output),
        format!("Ingestion key: {key}\nDATASET\tSTAGE\nmembers\tresolved\ndeclarations\tcleaned\n")
    );
    Ok(())
}

#[test]
fn missing_store_stays_absent() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    fs::write(tmp.path().join(".env"), "")?;
    fs::write(tmp.path().join("config.yaml"), "data_dir: ./data\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .current_dir(tmp.path())
        .env_remove("DATABASE_URL")
        .args(["data", "latest", "config.yaml"])
        .output()?;

    assert_eq!(stdout(&output), "No ingestion datasets found.\n");
    assert!(!tmp.path().join("data").exists());
    Ok(())
}

#[test]
fn ignores_empty_runs_unrelated_files_and_invalid_keys() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let data = tmp.path().join("data");
    let run = data.join(IngestionKey::default().to_string());
    fs::create_dir_all(run.join("raw/declarations"))?;
    fs::write(run.join("raw/declarations/notes.txt"), "not a dataset")?;
    fs::create_dir_all(run.join("raw/members.parquet"))?;
    fs::create_dir_all(data.join("not-an-ingestion-key/raw"))?;
    fs::write(
        data.join("not-an-ingestion-key/raw/members.parquet"),
        "ignored",
    )?;
    fs::write(
        data.join(IngestionKey::default().to_string()),
        "not a directory",
    )?;

    let output = run_latest(tmp.path())?;

    assert_eq!(stdout(&output), "No ingestion datasets found.\n");
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
