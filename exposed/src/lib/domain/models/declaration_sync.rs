//! Complete declaration evidence and its optimistic publication boundary.

use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    declaration_ingestion::{CapturedDeclaration, DeclarationId},
    parliament_member::MemberId,
};

/// Acquisition policy for a declaration run.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum DeclarationSyncMode {
    /// Discover changed IDs within the committed scope.
    Daily,
    /// Capture the complete current stored-member cohort.
    Reconcile,
}

/// Inclusive source-date bounds. The source is date based, not a changefeed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveryWindow {
    /// First source date, including overlap.
    pub from: NaiveDate,
    /// Last source date, excluding the configured indexing lag.
    pub through: NaiveDate,
}

/// A complete source record and its checked registrant.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OwnedDeclaration {
    /// Stored ownership prerequisite.
    pub member: MemberId,
    /// Complete unfiltered source record.
    pub declaration: CapturedDeclaration,
}

/// Source page evidence retained with a completed capture.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveryReceipt {
    /// Query URL, including offset and bounds.
    pub url: String,
    /// SHA-256 of the response bytes.
    pub sha256: String,
    /// Reported source total.
    pub total: usize,
    /// IDs and registrants observed on this page.
    pub declarations: Vec<(DeclarationId, MemberId)>,
}

/// Immutable complete capture. A manifest exists only after every request and file succeeds.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompleteDeclarationSnapshot {
    /// Format version.
    pub schema_version: u32,
    /// Stable run identity.
    pub run: Uuid,
    /// Publication head observed before acquisition.
    pub expected_revision: i64,
    /// Acquisition mode.
    pub mode: DeclarationSyncMode,
    /// Exact source endpoint and policy.
    pub source: String,
    /// Inclusive coverage bounds.
    pub window: DiscoveryWindow,
    /// Explicit cohort, including members with no declarations.
    pub scope: Vec<MemberId>,
    /// SHA-256 of the sorted member-ID cohort.
    pub scope_sha256: String,
    /// Stored members awaiting explicit historical declaration reconciliation.
    pub pending_members: Vec<MemberId>,
    /// Full evidence indexed by declaration ID.
    pub declarations: BTreeMap<DeclarationId, OwnedDeclaration>,
    /// Out-of-scope discoveries excluded from this capture.
    pub exclusions: BTreeMap<DeclarationId, MemberId>,
    /// Previously captured IDs omitted by a successful source request.
    pub omissions: Vec<DeclarationId>,
    /// All required source page receipts.
    pub receipts: Vec<DiscoveryReceipt>,
    /// Raw partition hashes, relative to the run directory.
    pub artifacts: BTreeMap<PathBuf, String>,
    /// Equality of complete source records, excluding retrieval times.
    pub content_sha256: String,
    /// Whether interpretation must run.
    pub content_changed: bool,
    /// Time at which all evidence was acquired.
    pub captured_at: DateTime<Utc>,
}

/// Database-selected committed capture, never inferred from filesystem recency.
#[derive(Clone, Debug)]
pub struct DeclarationSyncHead {
    /// Current publication revision.
    pub revision: i64,
    /// Sealed manifest selected by the database.
    pub manifest_path: PathBuf,
    /// Committed manifest digest.
    pub manifest_sha256: String,
}

/// Publication request checked under the declaration loader lock.
#[derive(Clone, Debug)]
pub struct PreparedPublication {
    /// Stable retry identity.
    pub run: Uuid,
    /// Expected committed revision.
    pub expected_revision: i64,
    /// Sealed immutable manifest.
    pub manifest_path: PathBuf,
    /// Digest binding the request to its evidence.
    pub manifest_sha256: String,
    /// Inclusive discovery checkpoint.
    pub coverage_date: NaiveDate,
}
