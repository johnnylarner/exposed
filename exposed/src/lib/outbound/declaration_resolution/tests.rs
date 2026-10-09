use super::*;
use crate::domain::models::entity_ingestion::IngestionKey;

#[tokio::test]
async fn publication_is_complete_and_refuses_existing_result() {
    let directory = tempfile::tempdir().unwrap();
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &directory.path().to_path_buf(),
        IngestionKey::default(),
    )
    .unwrap();
    let result = ResolvedDeclarations {
        observations: vec![],
        attributions: vec![],
        pairs: vec![],
        manifest: serde_json::json!({"schema_version":1}),
    };
    storage.publish_resolution(&result).await.unwrap();
    let destination = storage.resolved_declarations_path();
    for name in [
        "observation_resolution.parquet",
        "payment_attribution.parquet",
        "pair_decisions.parquet",
        "manifest.json",
    ] {
        assert!(destination.join(name).is_file());
    }
    assert!(storage.publish_resolution(&result).await.is_err());
    assert_eq!(
        fs::read(destination.join("manifest.json")).await.unwrap(),
        serde_json::to_vec_pretty(&result.manifest).unwrap()
    );
}

#[test]
fn atomic_directory_publication_refuses_an_existing_empty_directory() {
    let temporary = tempfile::tempdir().unwrap();
    let staging = temporary.path().join("staging");
    let destination = temporary.path().join("destination");
    std::fs::create_dir(&staging).unwrap();
    std::fs::write(staging.join("evidence"), "retained").unwrap();
    std::fs::create_dir(&destination).unwrap();
    assert!(publish_directory(&staging, &destination).is_err());
    assert!(staging.join("evidence").is_file());
    assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 0);
}

#[tokio::test]
async fn existing_output_failure_preserves_file_and_leaves_no_staging() {
    let temporary = tempfile::tempdir().unwrap();
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &temporary.path().to_path_buf(),
        IngestionKey::default(),
    )
    .unwrap();
    let destination = storage.resolved_declarations_path();
    let parent = destination.parent().unwrap();
    fs::create_dir_all(parent).await.unwrap();
    fs::write(&destination, b"existing file").await.unwrap();
    let result = ResolvedDeclarations {
        observations: vec![],
        attributions: vec![],
        pairs: vec![],
        manifest: serde_json::json!({}),
    };
    assert!(storage.publish_resolution(&result).await.is_err());
    assert_eq!(fs::read(&destination).await.unwrap(), b"existing file");
    assert_eq!(std::fs::read_dir(parent).unwrap().count(), 1);
}
