//! Typed storage contract for complete, immutable member captures.
use crate::domain::models::member_ingestion::{CaptureId, MemberCapture};
use thiserror::Error;

/// Local raw capture storage shared by Fetch and Load.
pub trait EntityIngestionStorage: Clone + Send + Sync + 'static {
    /// Read the explicitly selected completed capture and verify its manifest and datasets.
    fn read_member_capture(
        &self,
        id: CaptureId,
    ) -> impl Future<Output = Result<MemberCapture, EntitySearchPipelineError>> + Send;
    /// Finalize every dataset before atomically exposing this capture.
    fn write_member_capture(
        &self,
        capture: MemberCapture,
    ) -> impl Future<Output = Result<CaptureId, EntitySearchPipelineError>> + Send;
}

/// Filesystem, Parquet or manifest failure.
#[derive(Error, Debug)]
#[error("{0}")]
pub struct EntitySearchPipelineError(pub String);
