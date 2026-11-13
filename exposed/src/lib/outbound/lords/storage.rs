use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::{
    models::{
        entity_ingestion::IngestionKey,
        lords::{CompleteLordsCapture, LordsCaptureBuilder, LordsError},
    },
    repositories::lords::LordsCaptureStorage,
};

/// Files for one ingestion identity. Publication never replaces existing evidence.
pub struct LordsCaptureFiles {
    path: PathBuf,
    key: IngestionKey,
}

impl LordsCaptureFiles {
    /// Selects a run without creating or mutating its files.
    ///
    /// # Errors
    /// Returns an error if the root cannot be made absolute.
    pub fn new(root: &Path, key: IngestionKey) -> Result<Self, LordsError> {
        Ok(Self {
            path: std::path::absolute(root)?
                .join(key.to_string())
                .join("raw/lords"),
            key,
        })
    }
    /// Absolute artifact directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    format_version: u32,
    ingestion_key: uuid::Uuid,
    captured_at: DateTime<Utc>,
    source: String,
    include_deleted: bool,
    members: usize,
    pages: Vec<PageDigest>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageDigest {
    page: usize,
    bytes: usize,
    sha256: String,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn manifest(capture: &CompleteLordsCapture) -> Manifest {
    Manifest {
        format_version: 1,
        ingestion_key: capture.key().uuid(),
        captured_at: capture.captured_at(),
        source: super::REGISTER_URL.into(),
        include_deleted: true,
        members: capture.members().len(),
        pages: capture
            .pages()
            .iter()
            .enumerate()
            .map(|(index, bytes)| PageDigest {
                page: index + 1,
                bytes: bytes.len(),
                sha256: digest(bytes),
            })
            .collect(),
    }
}

pub(super) fn capture_digest(capture: &CompleteLordsCapture) -> Result<String, LordsError> {
    serde_json::to_vec(&manifest(capture))
        .map(|bytes| digest(&bytes))
        .map_err(|error| LordsError::Capture(error.to_string()))
}

impl LordsCaptureStorage for LordsCaptureFiles {
    async fn publish(&self, capture: &CompleteLordsCapture) -> Result<(), LordsError> {
        if capture.key() != &self.key {
            return Err(LordsError::Capture(
                "capture identity does not match destination".into(),
            ));
        }
        let parent = self.path.parent().expect("artifact has parent");
        tokio::fs::create_dir_all(parent).await?;
        let staging = tempfile::tempdir_in(parent)?;
        for (index, bytes) in capture.pages().iter().enumerate() {
            tokio::fs::write(
                staging.path().join(format!("page-{}.json", index + 1)),
                bytes,
            )
            .await?;
        }
        let bytes = serde_json::to_vec_pretty(&manifest(capture))
            .map_err(|error| LordsError::Capture(error.to_string()))?;
        tokio::fs::write(staging.path().join("manifest.json"), bytes).await?;
        crate::outbound::file_system::publish_directory(staging.path(), &self.path)?;
        Ok(())
    }

    async fn read(&self) -> Result<CompleteLordsCapture, LordsError> {
        let manifest: Manifest =
            serde_json::from_slice(&tokio::fs::read(self.path.join("manifest.json")).await?)
                .map_err(|error| LordsError::Capture(error.to_string()))?;
        if manifest.format_version != 1
            || manifest.ingestion_key != self.key.uuid()
            || manifest.source != super::REGISTER_URL
            || !manifest.include_deleted
            || manifest.pages.is_empty()
        {
            return Err(LordsError::Capture(
                "unsupported or mismatched Lords manifest".into(),
            ));
        }
        let mut builder = LordsCaptureBuilder::new(self.key.clone(), manifest.captured_at);
        for (index, page) in manifest.pages.iter().enumerate() {
            if page.page != index + 1 {
                return Err(LordsError::Capture("non-contiguous manifest pages".into()));
            }
            let bytes = tokio::fs::read(self.path.join(format!("page-{}.json", page.page))).await?;
            if bytes.len() != page.bytes || digest(&bytes) != page.sha256 {
                return Err(LordsError::Capture(format!(
                    "page {} digest mismatch",
                    page.page
                )));
            }
            builder.push_page(bytes)?;
        }
        let capture = builder.finish()?;
        if capture.members().len() != manifest.members {
            return Err(LordsError::Capture("manifest member count mismatch".into()));
        }
        Ok(capture)
    }
}
