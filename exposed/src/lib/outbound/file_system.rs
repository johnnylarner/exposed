//! File system interface for entity search ingestion

use std::path::{self, PathBuf};

use arrow::datatypes::{DataType, Field, Schema};
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
    /// Creates a new instance of [`ExposedDataPipeline`]
    ///
    /// # Errors
    /// - if [`root`] cannot be parsed as absolute
    /// - if required directories cannot be created
    pub fn new(root: &PathBuf) -> Result<Self, EntitySearchPipelineError> {
        let root_abs = path::absolute(root).map_err(|_| {
            EntitySearchPipelineError::Unexpected("valid root path must be provided".to_string())
        })?;

        let res = Self {
            root: root_abs,
            key: PipelineKey::new(),
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

pub(super) fn members_schema() -> Schema {
    let pmi = Field::new("parliament_member_id", DataType::UInt32, false);
    let n = Field::new("name", DataType::Utf8, false);
    let pi = Field::new("party_id", DataType::UInt32, false);
    let pn = Field::new("party_name", DataType::Utf8, false);
    let c = Field::new("constituency", DataType::Utf8, false);
    Schema::new(vec![pmi, n, pi, pn, c])
}
