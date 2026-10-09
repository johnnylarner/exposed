use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, SystemTime},
};

use exposed::{domain::models::entity_ingestion::IngestionKey, outbound::ExposedDataPipeline};
use tempfile::TempDir;

fn run_copy(root: &Path, key: Option<&IngestionKey>) -> anyhow::Result<Output> {
    fs::write(root.join(".env"), "")?;
    fs::write(root.join("config.yaml"), "data_dir: ./data\n")?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_exposed"));
    command.current_dir(root).env_remove("DATABASE_URL").args([
        "data",
        "copy-latest-raw",
        "--config",
        "config.yaml",
    ]);
    if let Some(key) = key {
        command.args(["--ingestion-key", &key.to_string()]);
    }
    Ok(command.output()?)
}

fn success(output: &Output) -> &str {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::str::from_utf8(&output.stdout).unwrap()
}

fn create_run(root: &Path, key: &IngestionKey, seconds: u64) -> anyhow::Result<PathBuf> {
    let path = root.join("data").join(key.to_string());
    fs::create_dir_all(path.join("raw"))?;
    fs::File::open(&path)?.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))?;
    Ok(path)
}

#[tokio::test]
async fn latest_raw_copies_nested_bytes_and_empty_directories_without_derived_stages()
-> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let older: IngestionKey = "ffffffff-ffff-4fff-8fff-ffffffffffff".parse()?;
    let source: IngestionKey = "00000000-0000-4000-8000-000000000001".parse()?;
    let destination = IngestionKey::default();
    let older_path = create_run(tmp.path(), &older, 10)?;
    fs::write(older_path.join("raw/older"), "wrong source")?;
    let source_path = create_run(tmp.path(), &source, 20)?;
    fs::create_dir_all(source_path.join("raw/members/nested/empty"))?;
    fs::create_dir_all(source_path.join("raw/declarations"))?;
    fs::write(
        source_path.join("raw/members/nested/capture"),
        [0, 255, 1, 128],
    )?;
    fs::write(source_path.join("raw/declarations/empty.parquet"), [])?;
    for stage in ["cleaned", "resolved"] {
        fs::create_dir(source_path.join(stage))?;
        fs::write(source_path.join(stage).join("derived"), "do not copy")?;
    }
    fs::File::open(&source_path)?.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(20))?;
    let data = tmp.path().join("data");
    let published = data.join(destination.to_string());
    let (path, selected) = ExposedDataPipeline::copy_latest_raw(&data, destination)
        .await?
        .unwrap();
    assert_eq!(path, published);
    assert_eq!(selected, source);
    assert_eq!(
        fs::read(published.join("raw/members/nested/capture"))?,
        [0, 255, 1, 128]
    );
    assert!(published.join("raw/members/nested/empty").is_dir());
    assert!(fs::read(published.join("raw/declarations/empty.parquet"))?.is_empty());
    assert_eq!(fs::read_dir(&published)?.count(), 1);
    assert!(!published.join("raw/older").exists());
    assert_eq!(
        fs::read(source_path.join("raw/members/nested/capture"))?,
        [0, 255, 1, 128]
    );
    assert_eq!(fs::read_dir(data)?.count(), 3);
    Ok(())
}

#[test]
fn cli_defaults_to_a_fresh_uuid_v7_and_reports_source_destination_and_path() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let source = IngestionKey::default();
    let source_path = create_run(tmp.path(), &source, 10)?;
    fs::write(source_path.join("raw/capture"), "retained")?;
    let output = run_copy(tmp.path(), None)?;
    let text = success(&output);
    let key: IngestionKey = text
        .lines()
        .find_map(|line| line.strip_prefix("Ingestion key: "))
        .unwrap()
        .parse()?;
    assert_ne!(key, source);
    assert_eq!(key.uuid().get_version_num(), 7);
    let path = tmp
        .path()
        .join("data")
        .canonicalize()?
        .join(key.to_string());
    assert_eq!(
        text,
        format!(
            "Source ingestion key: {source}\nIngestion key: {key}\nPath: {}\n",
            path.display()
        )
    );
    assert_eq!(fs::read_to_string(path.join("raw/capture"))?, "retained");
    Ok(())
}

#[test]
fn cli_copies_into_an_explicit_arbitrary_uuid() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let source = IngestionKey::default();
    let destination: IngestionKey = "00000000-0000-4000-8000-000000000001".parse()?;
    let path = create_run(tmp.path(), &source, 10)?;
    fs::write(path.join("raw/capture"), "retained")?;
    let output = run_copy(tmp.path(), Some(&destination))?;
    assert!(success(&output).contains(&format!("Ingestion key: {destination}\n")));
    assert_eq!(
        fs::read_to_string(
            tmp.path()
                .join("data")
                .join(destination.to_string())
                .join("raw/capture")
        )?,
        "retained"
    );
    Ok(())
}

#[test]
fn missing_or_empty_store_returns_no_runs_without_mutation() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let destination = IngestionKey::default();
    assert_eq!(
        success(&run_copy(tmp.path(), Some(&destination))?),
        "No ingestion runs found.\n"
    );
    let data = tmp.path().join("data");
    assert!(!data.exists());
    fs::create_dir(&data)?;
    assert_eq!(
        success(&run_copy(tmp.path(), None)?),
        "No ingestion runs found.\n"
    );
    assert_eq!(fs::read_dir(data)?.count(), 0);
    Ok(())
}

#[test]
fn missing_latest_raw_fails_without_falling_back_or_publishing() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let older = IngestionKey::default();
    create_run(tmp.path(), &older, 10)?;
    let source = IngestionKey::default();
    let path = create_run(tmp.path(), &source, 20)?;
    fs::remove_dir(path.join("raw"))?;
    let destination = IngestionKey::default();
    let output = run_copy(tmp.path(), Some(&destination))?;
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(&source.to_string()));
    assert_eq!(fs::read_dir(tmp.path().join("data"))?.count(), 2);
    Ok(())
}

#[test]
fn existing_destinations_including_empty_and_source_run_survive_unchanged() -> anyhow::Result<()> {
    for populated in [false, true] {
        let tmp = TempDir::new()?;
        let source = IngestionKey::default();
        let destination = IngestionKey::default();
        let source_path = create_run(tmp.path(), &source, 20)?;
        fs::write(source_path.join("raw/capture"), "retained")?;
        let path = tmp.path().join("data").join(destination.to_string());
        fs::create_dir(&path)?;
        if populated {
            fs::write(path.join("marker"), "untouched")?;
        }
        fs::File::open(&path)?.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(10))?;
        for key in [&destination, &source] {
            let output = run_copy(tmp.path(), Some(key))?;
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert_eq!(
                fs::read_to_string(source_path.join("raw/capture"))?,
                "retained"
            );
            assert_eq!(fs::read_dir(&path)?.count(), usize::from(populated));
            if populated {
                assert_eq!(fs::read_to_string(path.join("marker"))?, "untouched");
            }
            assert_eq!(fs::read_dir(tmp.path().join("data"))?.count(), 2);
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlinks_at_raw_root_or_inside_raw_fail_without_publishing_or_removing_other_staging()
-> anyhow::Result<()> {
    use std::os::unix::fs::symlink;
    for root_symlink in [false, true] {
        let tmp = TempDir::new()?;
        let source = IngestionKey::default();
        let path = create_run(tmp.path(), &source, 10)?;
        let external = tmp.path().join("external");
        fs::create_dir(&external)?;
        fs::write(external.join("capture"), "external")?;
        if root_symlink {
            fs::remove_dir(path.join("raw"))?;
            symlink(&external, path.join("raw"))?;
        } else {
            fs::write(path.join("raw/valid"), "retained")?;
            symlink(external.join("capture"), path.join("raw/link"))?;
        }
        let abandoned = tmp.path().join("data/.raw-copy-abandoned");
        fs::create_dir(&abandoned)?;
        fs::write(abandoned.join("marker"), "untouched")?;
        let output = run_copy(tmp.path(), Some(&IngestionKey::default()))?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("symlink"));
        assert_eq!(fs::read_dir(tmp.path().join("data"))?.count(), 2);
        assert_eq!(fs::read_to_string(external.join("capture"))?, "external");
        assert_eq!(fs::read_to_string(abandoned.join("marker"))?, "untouched");
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn special_file_fails_without_publishing() -> anyhow::Result<()> {
    use std::os::unix::net::UnixListener;
    let tmp = TempDir::new_in("/tmp")?;
    let source = IngestionKey::default();
    let path = create_run(tmp.path(), &source, 10)?;
    let _socket = UnixListener::bind(path.join("raw/socket"))?;
    let output = run_copy(tmp.path(), Some(&IngestionKey::default()))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("special file"));
    assert_eq!(fs::read_dir(tmp.path().join("data"))?.count(), 1);
    Ok(())
}
