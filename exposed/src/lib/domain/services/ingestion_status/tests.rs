use std::{
    future::ready,
    time::{Duration, SystemTime},
};

use uuid::Uuid;

use super::*;
use crate::domain::{
    models::{
        entity_ingestion::IngestionKey,
        ingestion_status::{IngestionDataStage, IngestionDataset, StoredIngestionDataset},
    },
    repositories::entity_ingestion::EntitySearchPipelineError,
};

struct Catalog(Vec<StoredIngestionDataset>);

impl IngestionCatalog for Catalog {
    fn stored_datasets(
        &self,
    ) -> impl Future<Output = Result<Vec<StoredIngestionDataset>, EntitySearchPipelineError>> + Send
    {
        ready(Ok(self.0.clone()))
    }
}

fn stored(
    key: u128,
    dataset: IngestionDataset,
    stage: IngestionDataStage,
    seconds: u64,
) -> StoredIngestionDataset {
    StoredIngestionDataset {
        key: IngestionKey::new(Uuid::from_u128(key)),
        dataset,
        stage,
        modified_at: SystemTime::UNIX_EPOCH + Duration::from_secs(seconds),
    }
}

#[tokio::test]
async fn latest_uses_file_time_and_reports_furthest_stages_only_within_that_run()
-> anyhow::Result<()> {
    use IngestionDataStage::{Cleaned, Raw, Resolved};
    use IngestionDataset::{Declarations, Members};

    let service = IngestionStatusService::new(Catalog(vec![
        stored(99, Members, Resolved, 10),
        stored(1, Declarations, Cleaned, 20),
        stored(1, Members, Resolved, 15),
        stored(1, Declarations, Raw, 30),
        stored(1, Members, Raw, 14),
    ]));

    assert_eq!(
        service.latest().await?,
        Some(IngestionStatus {
            key: IngestionKey::new(Uuid::from_u128(1)),
            datasets: vec![(Members, Resolved), (Declarations, Cleaned)],
        })
    );
    Ok(())
}

#[tokio::test]
async fn equal_timestamps_choose_the_same_run_regardless_of_catalog_order() -> anyhow::Result<()> {
    let datasets = vec![
        stored(1, IngestionDataset::Members, IngestionDataStage::Raw, 10),
        stored(
            2,
            IngestionDataset::Declarations,
            IngestionDataStage::Raw,
            10,
        ),
    ];
    let forward = IngestionStatusService::new(Catalog(datasets.clone()))
        .latest()
        .await?;
    let reverse = IngestionStatusService::new(Catalog(datasets.into_iter().rev().collect()))
        .latest()
        .await?;

    assert_eq!(forward, reverse);
    assert_eq!(forward.unwrap().key, IngestionKey::new(Uuid::from_u128(2)));
    Ok(())
}

#[tokio::test]
async fn empty_catalog_has_no_latest_run() -> anyhow::Result<()> {
    assert_eq!(
        IngestionStatusService::new(Catalog(vec![]))
            .latest()
            .await?,
        None
    );
    Ok(())
}

#[tokio::test]
async fn read_failure_is_not_reported_as_an_empty_catalog() {
    struct UnreadableCatalog;
    impl IngestionCatalog for UnreadableCatalog {
        fn stored_datasets(
            &self,
        ) -> impl Future<Output = Result<Vec<StoredIngestionDataset>, EntitySearchPipelineError>> + Send
        {
            ready(Err(EntitySearchPipelineError::ReadError(
                "permission denied".into(),
            )))
        }
    }

    assert!(matches!(
        IngestionStatusService::new(UnreadableCatalog).latest().await,
        Err(EntityIngestionError::IoError(EntitySearchPipelineError::ReadError(message))) if message == "permission denied"
    ));
}
