//! Filesystem catalog for stored ingestion datasets.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::SystemTime,
};

use tokio::fs;

use crate::domain::{
    models::{
        entity_ingestion::IngestionKey,
        ingestion_status::{IngestionDataStage, IngestionDataset, StoredIngestionDataset},
    },
    repositories::{
        entity_ingestion::EntitySearchPipelineError, ingestion_catalog::IngestionCatalog,
    },
};

/// Read-only catalog of UUID-named ingestion directories.
pub struct ExposedIngestionCatalog {
    root: PathBuf,
}

impl ExposedIngestionCatalog {
    /// Creates a catalog without changes to the filesystem.
    #[must_use]
    pub const fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl IngestionCatalog for ExposedIngestionCatalog {
    async fn stored_datasets(
        &self,
    ) -> Result<Vec<StoredIngestionDataset>, EntitySearchPipelineError> {
        let mut entries = match fs::read_dir(&self.root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(read_error(&self.root, error)),
        };
        let mut datasets = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| read_error(&self.root, e))?
        {
            let path = entry.path();
            if !entry
                .file_type()
                .await
                .map_err(|e| read_error(&path, e))?
                .is_dir()
            {
                continue;
            }
            let Some(key) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<IngestionKey>().ok())
            else {
                continue;
            };
            for (stage, directory) in [
                (IngestionDataStage::Raw, "raw"),
                (IngestionDataStage::Cleaned, "cleaned"),
                (IngestionDataStage::Resolved, "resolved"),
            ] {
                let stage_path = path.join(directory);
                for (dataset, modified_at) in [
                    (
                        IngestionDataset::Members,
                        file_modified_at(&stage_path.join("members.parquet")).await?,
                    ),
                    (
                        IngestionDataset::Declarations,
                        declarations_modified_at(&stage_path.join("declarations")).await?,
                    ),
                ] {
                    if let Some(modified_at) = modified_at {
                        datasets.push(StoredIngestionDataset {
                            key: key.clone(),
                            dataset,
                            stage,
                            modified_at,
                        });
                    }
                }
            }
        }
        Ok(datasets)
    }
}

async fn file_modified_at(path: &Path) -> Result<Option<SystemTime>, EntitySearchPipelineError> {
    match fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.is_file() => metadata
            .modified()
            .map(Some)
            .map_err(|e| read_error(path, e)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(read_error(path, error)),
    }
}

async fn declarations_modified_at(
    path: &Path,
) -> Result<Option<SystemTime>, EntitySearchPipelineError> {
    let mut entries = match fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(read_error(path, error)),
    };
    let mut latest = None;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| read_error(path, e))?
    {
        let file_path = entry.path();
        if file_path
            .extension()
            .is_some_and(|extension| extension == "parquet")
        {
            latest = latest.max(file_modified_at(&file_path).await?);
        }
    }
    Ok(latest)
}

fn read_error(path: &Path, error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::ReadError(format!("{}: {error}", path.display()))
}
