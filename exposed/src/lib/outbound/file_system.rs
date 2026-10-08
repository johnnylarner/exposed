//! File system interface for entity search ingestion

use std::{
    io::ErrorKind,
    path::{self, Path, PathBuf},
    time::SystemTime,
};

use arrow::datatypes::{DataType, Field, Schema};
use tokio::fs;
use uuid::Uuid;

use crate::domain::{
    models::entity_ingestion::IngestionKey,
    repositories::entity_ingestion::EntitySearchPipelineError,
};

/// File system pipeline
#[derive(Clone)]
pub struct ExposedDataPipeline {
    root: PathBuf,
    key: PipelineKey,
    fs_schema: FsSchema,
}

#[derive(Clone)]
pub struct PipelineKey(Uuid);

impl PipelineKey {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl From<&PipelineKey> for IngestionKey {
    fn from(value: &PipelineKey) -> Self {
        Self::new(value.0)
    }
}

impl From<IngestionKey> for PipelineKey {
    fn from(value: IngestionKey) -> Self {
        Self(value.uuid())
    }
}

#[derive(Clone)]
struct FsSchema {
    raw: String,
    // cleaned: String,
    // resolved: String,
}

impl Default for FsSchema {
    fn default() -> Self {
        Self {
            raw: "raw".into(),
            // cleaned: "cleaned".into(),
            // resolved: "resolved".into(),
        }
    }
}

impl ExposedDataPipeline {
    /// Opens an existing raw declaration dataset without creating directories.
    ///
    /// # Errors
    /// Returns a read error for a missing or invalid declaration directory.
    pub fn open_existing_declarations(
        root: &Path,
        key: IngestionKey,
    ) -> Result<Self, EntitySearchPipelineError> {
        let root = path::absolute(root).map_err(|e| read_error(root, e))?;
        let directory = root.join(key.to_string()).join("raw/declarations");
        if !std::fs::metadata(&directory)
            .map_err(|e| read_error(&directory, e))?
            .is_dir()
        {
            return Err(read_error(&directory, "expected a declaration directory"));
        }
        Ok(Self {
            root,
            key: PipelineKey::from(key),
            fs_schema: FsSchema::default(),
        })
    }

    /// Opens an existing cleaned declaration dataset without requiring retained raw partitions.
    ///
    /// # Errors
    /// Returns a read error when the cleaned declaration directory is missing.
    pub fn open_cleaned_declarations(
        root: &Path,
        key: IngestionKey,
    ) -> Result<Self, EntitySearchPipelineError> {
        let root = path::absolute(root).map_err(|error| read_error(root, error))?;
        let directory = root.join(key.to_string()).join("cleaned/declarations");
        if !std::fs::metadata(&directory)
            .map_err(|error| read_error(&directory, error))?
            .is_dir()
        {
            return Err(read_error(
                &directory,
                "expected cleaned declaration directory",
            ));
        }
        Ok(Self {
            root,
            key: PipelineKey::from(key),
            fs_schema: FsSchema::default(),
        })
    }

    pub(super) fn resolved_declarations_path(&self) -> PathBuf {
        self.latest_run_path().join("resolved/declarations")
    }

    pub(super) fn cleaned_declarations_path(&self) -> PathBuf {
        self.latest_run_path().join("cleaned/declarations")
    }

    /// Returns the absolute path and key of the most recently modified run directory.
    /// Equal timestamps use the greatest UUID. An absent or empty store returns `None`.
    /// This operation does not create or change files.
    ///
    /// # Errors
    /// Returns an error if ingestion storage cannot be read.
    pub async fn latest_ingestion(
        root: &Path,
    ) -> Result<Option<(PathBuf, IngestionKey)>, EntitySearchPipelineError> {
        let root = path::absolute(root).map_err(|e| read_error(root, e))?;
        let mut entries = match fs::read_dir(&root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(read_error(&root, error)),
        };
        let mut latest: Option<(SystemTime, PathBuf, IngestionKey)> = None;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| read_error(&root, e))?
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
            let modified_at = entry
                .metadata()
                .await
                .and_then(|metadata| metadata.modified())
                .map_err(|e| read_error(&path, e))?;
            if latest.as_ref().is_none_or(|(latest_time, _, latest_key)| {
                (modified_at, key.uuid()) > (*latest_time, latest_key.uuid())
            }) {
                latest = Some((modified_at, path, key));
            }
        }
        Ok(latest.map(|(_, path, key)| (path, key)))
    }

    /// Creates a new instance of [`ExposedDataPipeline`] with an [`IngestionKey`]
    ///
    /// # Errors
    /// - if [`root`] cannot be parsed as absolute
    /// - if required directories cannot be created
    pub fn new_with_ingestion_key(
        root: &PathBuf,
        key: IngestionKey,
    ) -> Result<Self, EntitySearchPipelineError> {
        let root_abs = path::absolute(root).map_err(|_| {
            EntitySearchPipelineError::Unexpected("valid root path must be provided".to_string())
        })?;

        let res = Self {
            root: root_abs,
            key: PipelineKey::from(key),
            fs_schema: FsSchema::default(),
        };
        std::fs::create_dir_all(res.raw_path()).map_err(|_| {
            EntitySearchPipelineError::Unexpected("must be able to create fs paths".to_string())
        })?;
        Ok(res)
    }

    fn latest_run_path(&self) -> PathBuf {
        self.root.join(self.key.0.to_string())
    }

    pub(super) fn raw_path(&self) -> PathBuf {
        self.latest_run_path().join(&self.fs_schema.raw)
    }

    pub(super) const fn key(&self) -> &PipelineKey {
        &self.key
    }

    // pub(super) fn cleaned_path(&self) -> PathBuf {
    //     self.latest_run_path().join(&self.fs_schema.cleaned)
    // }
    //
    // pub(super) fn resolved_path(&self) -> PathBuf {
    //     self.latest_run_path().join(&self.fs_schema.resolved)
    // }
}

fn read_error(path: &Path, error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::ReadError(format!("{}: {error}", path.display()))
}

pub(super) fn members_schema() -> Schema {
    let pmi = Field::new("parliament_member_id", DataType::UInt32, false);
    let n = Field::new("name", DataType::Utf8, false);
    let pi = Field::new("party_id", DataType::UInt32, false);
    let pn = Field::new("party_name", DataType::Utf8, false);
    let c = Field::new("constituency", DataType::Utf8, false);
    Schema::new(vec![pmi, n, pi, pn, c])
}
