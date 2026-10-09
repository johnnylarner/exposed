//! Contains search entity models

use serde::Serialize;
use thiserror::Error;

use crate::domain::models::{funder::Funder as FunderDetails, parliament_member::ParliamentMember};

const MAX_STRICTNESS: f32 = 1_f32;
const MIN_STRICTNESS: f32 = 0_f32;
const MIN_ENTRIES: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// Name that supplied a search result's winning score.
pub enum SearchMatchSource {
    /// The canonical name won, including equal scores.
    Name,
    /// An observed alias scored higher than the canonical name.
    Alias {
        /// Original observed spelling.
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Canonical funder identity and its search match provenance.
pub struct FunderSearchMatch {
    funder: FunderDetails,
    source: SearchMatchSource,
}

impl FunderSearchMatch {
    #[must_use]
    /// Creates a search result for a canonical identity.
    pub const fn new(funder: FunderDetails, source: SearchMatchSource) -> Self {
        Self { funder, source }
    }

    #[must_use]
    /// Canonical funder.
    pub const fn funder(&self) -> &FunderDetails {
        &self.funder
    }

    #[must_use]
    /// Winning match source.
    pub const fn source(&self) -> &SearchMatchSource {
        &self.source
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Search entity kind
pub enum Entity {
    /// Company entity
    Funder(FunderSearchMatch),
    /// Member entitiy
    ParliamentMember(ParliamentMember),
}

impl Entity {
    #[must_use]
    /// Entity name
    pub fn name(&self) -> &str {
        match self {
            Self::Funder(kind) => kind.funder().name(),
            Self::ParliamentMember(mp) => mp.name(),
        }
    }

    #[must_use]
    /// Entity kind
    pub const fn kind(&self) -> &str {
        match self {
            Self::Funder(_) => "Funder",
            Self::ParliamentMember(_) => "MP",
        }
    }

    #[must_use]
    /// Funder kind
    pub fn funder_kind(&self) -> Option<&str> {
        match self {
            Self::Funder(kind) => Some(kind.funder().kind()),
            Self::ParliamentMember(_) => None,
        }
    }
}

impl From<&ParliamentMember> for Entity {
    fn from(value: &ParliamentMember) -> Self {
        Self::ParliamentMember(value.clone())
    }
}

impl From<&FunderSearchMatch> for Entity {
    fn from(value: &FunderSearchMatch) -> Self {
        Self::Funder(value.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Search entity name
pub struct EntityName(String);

#[derive(Clone, Debug, PartialEq)]
/// Data required to search entities
pub struct EntitySearchRequest {
    term: String,
    strictness: f32,
    max_entries: u8,
}

impl EntitySearchRequest {
    #[must_use]
    /// Search term
    pub const fn term(&self) -> &str {
        self.term.as_str()
    }

    #[must_use]
    /// Search strictness
    pub const fn strictness(&self) -> f32 {
        self.strictness
    }

    #[must_use]
    /// Max result set
    pub const fn max_entries(&self) -> u8 {
        self.max_entries
    }
}

impl EntitySearchRequest {
    /// Creates new instance with [`MAX_STRICTNESS`]
    ///
    /// # Errors
    /// - See [`Self::new_with_strictness`]
    pub fn new_strict(term: String, max_entries: u8) -> Result<Self, EntitySearchError> {
        Self::new_with_strictness(term, max_entries, MAX_STRICTNESS)
    }
    /// Creates new instance with variable strictness
    ///
    /// # Errors
    /// - Term too short
    /// - Too few entries
    /// - Invalid strictness
    pub fn new_with_strictness(
        term: String,
        max_entries: u8,
        strictness: f32,
    ) -> Result<Self, EntitySearchError> {
        // It will be a u8
        #[allow(clippy::cast_possible_truncation)]
        if term.trim().len() < 3 {
            return Err(EntitySearchError::InvalidTerm(term.trim().len() as u8));
        }
        if max_entries < MIN_ENTRIES {
            return Err(EntitySearchError::TooFewEntries);
        }
        if !strictness.is_finite() || strictness < MIN_STRICTNESS || strictness > MAX_STRICTNESS {
            return Err(EntitySearchError::InvalidStrictness(strictness));
        }
        Ok(Self {
            term,
            strictness,
            max_entries,
        })
    }
}

/// Errors related to entity search
#[derive(Error, Debug)]
pub enum EntitySearchError {
    /// Unexpected runtime errors, usually from the database
    #[error("unexpected error occurred: {0}")]
    UnexpectedError(String),

    /// Term too short for generating valid search results
    #[error("term must be at last three characters long; received {0}")]
    InvalidTerm(u8),

    /// Invalid strictness value
    #[error("at least one entry must be expected to return")]
    TooFewEntries,

    /// Invalid strictness value
    #[error("strictness must be between {MIN_STRICTNESS} and {MAX_STRICTNESS}; get {0}")]
    InvalidStrictness(f32),
}

#[cfg(test)]
mod strictness {
    use super::EntitySearchRequest;

    #[test]
    fn rejects_non_finite_thresholds() {
        for threshold in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                EntitySearchRequest::new_with_strictness("West".into(), 10, threshold).is_err()
            );
        }
    }
}
