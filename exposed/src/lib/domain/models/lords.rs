//! Complete, source-faithful captures of the Lords register.

mod source;

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::Serialize;
use strum::EnumIter;

use super::{entity_ingestion::IngestionKey, parliament_member::MemberId};

/// Shared source timestamp used by the register wire contract.
pub use super::source_timestamp::SourceTimestamp;

impl From<super::source_timestamp::SourceTimestampError> for LordsError {
    fn from(error: super::source_timestamp::SourceTimestampError) -> Self {
        Self::Source(error.to_string())
    }
}

/// Source category identity, including unrecognised categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct LordsCategoryId(std::num::NonZeroU32);

impl LordsCategoryId {
    /// Nonzero source identity, distinct from a stored occurrence position.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for LordsCategoryId {
    type Error = LordsError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        std::num::NonZeroU32::new(value)
            .map(Self)
            .ok_or_else(|| LordsError::Source("source identity must be nonzero".into()))
    }
}

/// Source interest identity. Repeated occurrences remain separate tree nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SimpleDeclarationId(std::num::NonZeroU32);

impl SimpleDeclarationId {
    /// Nonzero source identity, distinct from a stored occurrence position.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for SimpleDeclarationId {
    type Error = LordsError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        std::num::NonZeroU32::new(value)
            .map(Self)
            .ok_or_else(|| LordsError::Source("source identity must be nonzero".into()))
    }
}

/// The backend owns the category placement vocabulary and classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LordsPlacement {
    /// Current and historical shareholding categories.
    Shareholdings,
    /// Every remaining source category, including Nil and unknown categories.
    Other,
}

impl LordsCategoryId {
    /// Classifies all source versions of shareholding categories.
    #[must_use]
    pub const fn placement(self) -> LordsPlacement {
        match self.0.get() {
            4 | 12 | 31 | 1002 | 1003 | 1004 | 1005 => LordsPlacement::Shareholdings,
            _ => LordsPlacement::Other,
        }
    }
}

/// Canonical latest member profile used by the Lords transport.
pub use super::membership::MemberProfile as LordProfile;

/// One registered-interest occurrence with its complete child tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SimpleDeclaration {
    /// Original source identity, not an occurrence key.
    pub source_id: SimpleDeclarationId,
    /// Original text, including null and whitespace.
    pub text: Option<String>,
    /// Original source creation timestamp.
    pub created_when: Option<SourceTimestamp>,
    /// Original source amendment timestamp.
    pub last_amended_when: Option<SourceTimestamp>,
    /// Only this field identifies a deleted occurrence.
    pub deleted_when: Option<SourceTimestamp>,
    /// Correction status is independent of deletion.
    pub is_correction: bool,
    /// Ordered child occurrences, without deduplication.
    pub children: Vec<Self>,
}

/// One source category occurrence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LordsCategory {
    /// Original category identity.
    pub source_id: LordsCategoryId,
    /// Original category name, including null.
    pub name: Option<String>,
    /// Source display ordering.
    pub sort_order: i32,
    /// Canonical layout classification.
    pub placement: LordsPlacement,
    /// Ordered root occurrences, including deleted interests.
    pub interests: Vec<SimpleDeclaration>,
}

/// A canonical member profile and complete register from one capture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LordRegister {
    /// Shared canonical member profile.
    pub member: LordProfile,
    /// UTC capture provenance, distinct from source lifecycle dates.
    pub captured_at: DateTime<Utc>,
    /// Every category occurrence in source order.
    pub categories: Vec<LordsCategory>,
}

impl LordRegister {
    /// Parses the retained source document at the storage boundary.
    ///
    /// # Errors
    /// Rejects malformed identities, timestamps, or source structure.
    pub fn from_source(
        raw: &serde_json::Value,
        captured_at: DateTime<Utc>,
    ) -> Result<Self, LordsError> {
        source::register(raw, captured_at)
    }
}

/// A parsed source document alongside its unmodified item.value object.
pub struct CapturedLord {
    register: LordRegister,
    raw: serde_json::Value,
}

impl CapturedLord {
    /// Validated register.
    #[must_use]
    pub const fn register(&self) -> &LordRegister {
        &self.register
    }

    /// Full source record, including fields outside the public view.
    #[must_use]
    pub const fn raw(&self) -> &serde_json::Value {
        &self.raw
    }
}

/// Pagination in progress. Only finish can produce an importable capture.
pub struct LordsCaptureBuilder {
    key: IngestionKey,
    captured_at: DateTime<Utc>,
    total: Option<usize>,
    members: Vec<CapturedLord>,
    ids: BTreeSet<MemberId>,
    pages: Vec<Vec<u8>>,
}

impl LordsCaptureBuilder {
    /// Starts a fresh capture before the first request.
    #[must_use]
    pub const fn new(key: IngestionKey, captured_at: DateTime<Utc>) -> Self {
        Self {
            key,
            captured_at,
            total: None,
            members: Vec::new(),
            ids: BTreeSet::new(),
            pages: Vec::new(),
        }
    }

    /// Checks and retains one complete raw response in sequence.
    ///
    /// # Errors
    /// Rejects inconsistent totals, gaps, duplicate members, stalled or excess pages.
    pub fn push_page(&mut self, bytes: Vec<u8>) -> Result<(), LordsError> {
        let page = source::page(&bytes, self.captured_at)?;
        if self.is_complete()
            || page.skip != self.members.len()
            || page.take == 0
            || page.members.len() > page.take
            || self.total.is_some_and(|total| total != page.total)
            || page
                .skip
                .checked_add(page.members.len())
                .is_none_or(|end| end > page.total)
            || (page.members.is_empty() && page.total != 0)
            || (page.skip + page.members.len() < page.total && page.members.len() != page.take)
        {
            return Err(LordsError::Capture(
                "incomplete or inconsistent Lords pagination".into(),
            ));
        }
        let mut page_ids = BTreeSet::new();
        for member in &page.members {
            let id = member.register.member.id;
            if self.ids.contains(&id) || !page_ids.insert(id) {
                return Err(LordsError::Capture(format!(
                    "duplicate member {id} in Lords capture"
                )));
            }
        }
        self.total = Some(page.total);
        self.ids.extend(page_ids);
        self.members.extend(page.members);
        self.pages.push(bytes);
        Ok(())
    }

    /// Whether all reported member occurrences have been captured.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.total == Some(self.members.len())
    }

    /// Seals a complete capture, retaining member order for source comparison.
    ///
    /// # Errors
    /// A partial or unstarted capture cannot be published or loaded.
    pub fn finish(self) -> Result<CompleteLordsCapture, LordsError> {
        if !self.is_complete() {
            return Err(LordsError::Capture("Lords capture is incomplete".into()));
        }
        Ok(CompleteLordsCapture {
            key: self.key,
            captured_at: self.captured_at,
            members: self.members,
            pages: self.pages,
        })
    }
}

/// Only a fully checked capture may be persisted or loaded.
pub struct CompleteLordsCapture {
    key: IngestionKey,
    captured_at: DateTime<Utc>,
    members: Vec<CapturedLord>,
    pages: Vec<Vec<u8>>,
}

impl CompleteLordsCapture {
    /// Capture identity.
    #[must_use]
    pub const fn key(&self) -> &IngestionKey {
        &self.key
    }
    /// UTC start of capture.
    #[must_use]
    pub const fn captured_at(&self) -> DateTime<Utc> {
        self.captured_at
    }
    /// Every captured member document.
    #[must_use]
    pub fn members(&self) -> &[CapturedLord] {
        &self.members
    }
    /// Exact raw page response bytes.
    #[must_use]
    pub fn pages(&self) -> &[Vec<u8>] {
        &self.pages
    }
}

/// Lords import stages.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LordsIngestionStage {
    /// Capture every page, including deleted interests.
    Fetch,
    /// Load a complete capture transactionally.
    Load,
}

/// Result of an immutable capture load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LordsLoadOutcome {
    /// This capture was committed.
    Loaded,
    /// The same identity and artifact digest were previously committed.
    AlreadyLoaded,
}

/// Import failures retain diagnostics for CLI output and private server logs.
#[derive(Debug, thiserror::Error)]
pub enum LordsError {
    /// Invalid source response or transport failure.
    #[error("Lords source failed: {0}")]
    Source(String),
    /// Missing or inconsistent complete-capture evidence.
    #[error("invalid Lords capture: {0}")]
    Capture(String),
    /// Artifact access or publication failure.
    #[error("Lords artifact failed: {0}")]
    Artifact(#[from] std::io::Error),
    /// Database operation failure.
    #[error("Lords database failed: {0}")]
    Database(String),
    /// An ingestion identity has already been used for different evidence.
    #[error("Lords ingestion key already loaded with a different digest")]
    Conflict,
}

#[cfg(test)]
pub(crate) mod tests;
