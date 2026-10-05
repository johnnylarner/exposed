//! Read-only access to stored ingestion runs.

use super::entity_ingestion::EntitySearchPipelineError;
use crate::domain::models::ingestion_status::StoredIngestionDataset;

/// Catalog of datasets across ingestion runs.
pub trait IngestionCatalog: Send + Sync {
    /// Lists stored datasets, with one entry per run, dataset, and stage.
    /// Empty directories do not count as datasets. Order is unspecified.
    /// A missing store returns an empty list. Other read failures return an error.
    fn stored_datasets(
        &self,
    ) -> impl Future<Output = Result<Vec<StoredIngestionDataset>, EntitySearchPipelineError>> + Send;
}
