//! Inspect the latest ingestion run through its stored datasets.

use std::collections::BTreeMap;

use crate::domain::{
    models::{entity_ingestion::EntityIngestionError, ingestion_status::IngestionStatus},
    repositories::ingestion_catalog::IngestionCatalog,
};

#[cfg(test)]
mod tests;

/// Reports available datasets without changes to ingestion storage.
pub struct IngestionStatusService<C> {
    catalog: C,
}

impl<C: IngestionCatalog> IngestionStatusService<C> {
    /// Creates a service with the supplied catalog.
    pub const fn new(catalog: C) -> Self {
        Self { catalog }
    }

    /// Returns the run with the most recently modified dataset file.
    /// Equal timestamps use the greatest UUID as a deterministic tie-breaker.
    /// Each dataset reports its furthest stored stage within that run.
    /// Returns `None` if no stored datasets exist.
    ///
    /// # Errors
    /// Returns an error if the catalog cannot read ingestion storage.
    pub async fn latest(&self) -> Result<Option<IngestionStatus>, EntityIngestionError> {
        let stored = self.catalog.stored_datasets().await?;
        let Some(latest) = stored
            .iter()
            .max_by_key(|entry| (entry.modified_at, entry.key.uuid()))
        else {
            return Ok(None);
        };
        let key = latest.key.clone();
        let mut datasets = BTreeMap::new();
        for entry in stored.into_iter().filter(|entry| entry.key == key) {
            datasets
                .entry(entry.dataset)
                .and_modify(|stage| *stage = entry.stage.max(*stage))
                .or_insert(entry.stage);
        }
        Ok(Some(IngestionStatus {
            key,
            datasets: datasets.into_iter().collect(),
        }))
    }
}
