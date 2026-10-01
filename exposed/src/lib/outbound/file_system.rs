//! Immutable member captures, staged and published on the same filesystem.
use super::member_parquet;
use crate::domain::{
    models::member_ingestion::{CaptureContext, CaptureCounts, CaptureId, MemberCapture},
    repositories::entity_ingestion_pipline::{EntityIngestionStorage, EntitySearchPipelineError},
};
use anyhow::{Context, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

/// Local data root with captures at raw/members/<UUIDv7>.
#[derive(Clone)]
pub struct ExposedDataPipeline {
    root: PathBuf,
}
impl ExposedDataPipeline {
    /// Use this data root without creating files until Fetch is finalized.
    #[must_use]
    pub const fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn read_at(path: &Path, id: CaptureId) -> anyhow::Result<MemberCapture> {
        let manifest: Manifest = serde_json::from_reader(
            File::open(path.join("manifest.json")).context("missing completed manifest")?,
        )
        .context("invalid manifest")?;
        ensure!(
            manifest.schema_version == 1,
            "unsupported member schema version {}",
            manifest.schema_version
        );
        ensure!(
            manifest.context.capture_id == id,
            "manifest capture identity does not match {id}"
        );
        ensure!(
            manifest.completed_at >= manifest.context.started_at,
            "completion timestamp precedes capture start"
        );
        ensure!(
            manifest.context.term_start <= manifest.context.observation_date,
            "term starts after observation date"
        );
        ensure!(
            manifest.datasets == DatasetFiles::default(),
            "manifest must identify all three schema-v1 datasets"
        );
        let observations = member_parquet::read(path)?;
        ensure!(
            observations.counts() == manifest.counts,
            "dataset row counts disagree with manifest"
        );
        observations.validate()?;
        Ok(MemberCapture {
            context: manifest.context,
            observations,
        })
    }

    fn stage(&self, capture: &MemberCapture) -> anyhow::Result<PendingCapture> {
        capture.observations.validate()?;
        let id = capture.context.capture_id;
        let parent = self.root.join("raw/members");
        fs::create_dir_all(&parent)?;
        let destination = parent.join(id.to_string());
        ensure!(
            !destination.exists(),
            "completed capture {id} already exists"
        );
        let staging = tempfile::Builder::new()
            .prefix(&format!(".staging-{id}-"))
            .tempdir_in(&parent)?;
        member_parquet::write(staging.path(), &capture.observations)?;
        let manifest = Manifest {
            schema_version: 1,
            context: capture.context.clone(),
            completed_at: Utc::now(),
            datasets: DatasetFiles::default(),
            counts: capture.observations.counts(),
        };
        let file = File::create_new(staging.path().join("manifest.json"))?;
        serde_json::to_writer_pretty(&file, &manifest)?;
        file.sync_all()?;
        let verified = Self::read_at(staging.path(), id)?;
        ensure!(
            verified == *capture,
            "capture round-trip verification failed"
        );
        Ok(PendingCapture {
            staging,
            destination,
            id,
        })
    }
}
impl EntityIngestionStorage for ExposedDataPipeline {
    async fn read_member_capture(
        &self,
        id: CaptureId,
    ) -> Result<MemberCapture, EntitySearchPipelineError> {
        let path = self.root.join("raw/members").join(id.to_string());
        tokio::task::spawn_blocking(move || Self::read_at(&path, id))
            .await
            .map_err(|error| EntitySearchPipelineError(format!("read capture {id}: {error}")))?
            .map_err(|error| EntitySearchPipelineError(format!("capture {id}: {error:#}")))
    }
    async fn write_member_capture(
        &self,
        capture: MemberCapture,
    ) -> Result<CaptureId, EntitySearchPipelineError> {
        let id = capture.context.capture_id;
        let storage = self.clone();
        let pending = tokio::task::spawn_blocking(move || storage.stage(&capture))
            .await
            .map_err(|error| EntitySearchPipelineError(format!("stage capture {id}: {error}")))?
            .map_err(|error| EntitySearchPipelineError(format!("capture {id}: {error:#}")))?;
        pending
            .publish()
            .map_err(|error| EntitySearchPipelineError(format!("publish capture {id}: {error:#}")))
    }
}

struct PendingCapture {
    staging: tempfile::TempDir,
    destination: PathBuf,
    id: CaptureId,
}

impl PendingCapture {
    fn publish(self) -> std::io::Result<CaptureId> {
        // Dropping a cancelled staging task removes its temporary directory.
        // No await or fallible work follows atomic publication.
        fs::rename(self.staging.path(), &self.destination)?;
        Ok(self.id)
    }
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    schema_version: u32,
    #[serde(flatten)]
    context: CaptureContext,
    completed_at: DateTime<Utc>,
    datasets: DatasetFiles,
    counts: CaptureCounts,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct DatasetFiles {
    profiles: String,
    current_commons: String,
    house_memberships: String,
}
impl Default for DatasetFiles {
    fn default() -> Self {
        Self {
            profiles: "profiles.parquet".into(),
            current_commons: "current_commons.parquet".into(),
            house_memberships: "house_memberships.parquet".into(),
        }
    }
}
