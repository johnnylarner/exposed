//! File system interface for entity search ingestion

use std::path::{self, PathBuf};

use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct ExposedDataPipeline {
    root: PathBuf,
    key: DateTime<Utc>,
    fs_schema: FsSchema,
}

#[derive(Clone)]
struct FsSchema {
    raw: String,
    cleaned: String,
    resolved: String,
}

impl Default for FsSchema {
    fn default() -> Self {
        Self {
            raw: "raw".into(),
            cleaned: "cleaned".into(),
            resolved: "resolved".into(),
        }
    }
}

impl ExposedDataPipeline {
    /// Creates a new instance of [ExposedDatabase]
    pub async fn new(root: PathBuf, key: DateTime<Utc>) -> Self {
        let root_abs = path::absolute(&root).expect("valid root path must be provided");
        Self {
            root: root_abs,
            key,
            fs_schema: FsSchema::default(),
        }
    }

    fn latest_run_path(&self) -> PathBuf {
        self.root.join(self.key.to_string())
    }

    pub(super) fn raw_path(&self) -> PathBuf {
        self.latest_run_path().join(&self.fs_schema.raw)
    }

    pub(super) fn cleaned_path(&self) -> PathBuf {
        self.latest_run_path().join(&self.fs_schema.cleaned)
    }

    pub(super) fn resolved_path(&self) -> PathBuf {
        self.latest_run_path().join(&self.fs_schema.resolved)
    }
}
