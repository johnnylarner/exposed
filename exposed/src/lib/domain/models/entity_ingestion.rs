//! Typed requests and outcomes for independent ingestion operations.

use super::member_ingestion::{CaptureCounts, CaptureId, MemberImportSummary};
use crate::domain::repositories::{
    entity_ingestion_pipline::EntitySearchPipelineError, member_writer::MemberWriteError,
    parliament_api::ParliamentApiError,
};
use chrono::NaiveDate;
use serde::Serialize;
use std::str::FromStr;
use strum::EnumString;
use thiserror::Error;

/// A single explicitly selected ingestion operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityIngestionRequest {
    target: EntityIngestionTarget,
}

impl EntityIngestionRequest {
    /// Select member acquisition or offline loading.
    #[must_use]
    pub const fn new_members_request(stage: MemberIngestionStage) -> Self {
        Self {
            target: EntityIngestionTarget::Members(stage),
        }
    }
    /// Select an existing declaration stage (implementation is deferred).
    ///
    /// # Errors
    /// Returns an error for an unknown stage name.
    pub fn new_declaration_request(stage: &str) -> Result<Self, EntityIngestionError> {
        let stage = DeclarationIngestionStage::from_str(stage)
            .map_err(|_| EntityIngestionError::InvalidStage(stage.to_string()))?;
        Ok(Self {
            target: EntityIngestionTarget::Declaration(stage),
        })
    }
    /// Operation selected by this request.
    #[must_use]
    pub const fn target(&self) -> &EntityIngestionTarget {
        &self.target
    }
}

/// Ingestion targets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityIngestionTarget {
    /// Member capture or database publication.
    Members(MemberIngestionStage),
    /// Declaration stages remain deferred.
    Declaration(DeclarationIngestionStage),
}

/// Independent member operations with distinct inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberIngestionStage {
    /// Acquire source data for the configured Parliament.
    Fetch {
        /// Beginning of the Parliament cohort.
        term_start: NaiveDate,
    },
    /// Apply a completed capture entirely offline from Parliament.
    Load {
        /// Explicit `UUIDv7` of the saved input.
        capture_id: CaptureId,
    },
}

/// From what stage to run the declaration pipeline.
#[derive(Clone, Debug, PartialEq, Eq, EnumString)]
#[allow(missing_docs)]
#[strum(ascii_case_insensitive)]
pub enum DeclarationIngestionStage {
    Import,
    Clean,
    Resolve,
}

/// Success is returned only after finalization or transaction commit.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum EntityIngestionOutcome {
    /// Completed immutable source capture.
    Fetch {
        /// Completed capture identity.
        capture_id: CaptureId,
        /// Raw dataset row counts.
        counts: CaptureCounts,
    },
    /// Committed database refresh.
    Load {
        /// Explicit capture used for the refresh.
        capture_id: CaptureId,
        /// Captured term start.
        term_start: NaiveDate,
        /// Captured observation date.
        observation_date: NaiveDate,
        /// Committed member outcomes.
        summary: MemberImportSummary,
    },
}

/// Failures while acquiring, validating or publishing ingestion data.
#[derive(Error, Debug)]
pub enum EntityIngestionError {
    /// Source request or decoding failure.
    #[error("Parliament source: {0}")]
    ApiError(#[from] ParliamentApiError),
    /// Invalid source observations or service rules.
    #[error("invalid member data: {0}")]
    DataError(String),
    /// Capture storage failure.
    #[error("capture storage: {0}")]
    IoError(#[from] EntitySearchPipelineError),
    /// Atomic database refresh failed.
    #[error("member load: {0}")]
    DatabaseError(#[from] MemberWriteError),
    /// Invalid or unavailable stage.
    #[error("unsupported ingestion stage: {0}")]
    InvalidStage(String),
}
