use tempfile::TempDir;

use super::ExposedDataPipeline;
use crate::domain::{
    models::{entity_ingestion::IngestionKey, parliament_member::ParliamentMember},
    repositories::entity_ingestion::EntityIngestionStorage,
};

#[tokio::test]
async fn preserves_members_across_multiple_parquet_batches() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    // The Parquet reader returns at most 1,024 rows in each default batch.
    let members = (1..=2_050)
        .map(|id| {
            ParliamentMember::new(
                format!("Member {id}"),
                id,
                "Example party".into(),
                7,
                "Example constituency".into(),
            )
        })
        .collect::<Vec<_>>();

    storage.write_raw_members(&members).await?;
    let restored = storage.read_raw_members().await?;

    assert_eq!(restored, members);
    Ok(())
}

#[tokio::test]
async fn reads_an_empty_members_file() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;

    storage.write_raw_members(&[]).await?;

    assert!(storage.read_raw_members().await?.is_empty());
    Ok(())
}
