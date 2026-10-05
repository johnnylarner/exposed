//! Stored datasets and stages for ingestion runs.

use std::time::SystemTime;

use super::entity_ingestion::IngestionKey;

/// Dataset stored by an ingestion run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IngestionDataset {
    /// Parliamentary members.
    Members,
    /// Members' declarations.
    Declarations,
}

/// Stored data stages, ordered by progress through the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IngestionDataStage {
    /// Source data from Parliament.
    Raw,
    /// Cleaned source data.
    Cleaned,
    /// Data with resolved entities.
    Resolved,
}

/// Evidence of a stored dataset at one stage of an ingestion run.
#[derive(Clone, Debug)]
pub struct StoredIngestionDataset {
    /// Run that contains the dataset.
    pub key: IngestionKey,
    /// Kind of data stored.
    pub dataset: IngestionDataset,
    /// Stage of the stored data.
    pub stage: IngestionDataStage,
    /// Most recent modification time of the dataset's files.
    pub modified_at: SystemTime,
}

/// Available datasets in the latest ingestion run.
#[derive(Debug, PartialEq, Eq)]
pub struct IngestionStatus {
    /// Key shared by the reported datasets.
    pub key: IngestionKey,
    /// Furthest stored stage for each available dataset, in dataset order.
    pub datasets: Vec<(IngestionDataset, IngestionDataStage)>,
}
