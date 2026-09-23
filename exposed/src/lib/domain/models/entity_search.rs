use thiserror::Error;

use crate::domain::models::{funder::Funder as FunderDetails, parliament_member::ParliamentMember};

const MAX_STRICTNESS: f32 = 1_f32;
const MIN_STRICTNESS: f32 = 0_f32;
const MIN_ENTRIES: u8 = 1;

#[derive(Clone, Debug, PartialEq)]
/// Search entity kind
pub enum Entity {
    /// Company entity
    Funder(FunderDetails),
    /// Member entitiy
    ParliamentMember(ParliamentMember),
}

impl Entity {
    /// Entity name
    pub fn name(&self) -> &str {
        match self {
            Self::Funder(kind) => kind.name(),
            Self::ParliamentMember(mp) => mp.name(),
        }
    }

    /// Entity kind
    pub fn kind(&self) -> &str {
        match self {
            Self::Funder(_) => "Funder",
            Self::ParliamentMember(_) => "MP",
        }
    }

    /// Funder kind
    pub fn funder_kind(&self) -> Option<&str> {
        match self {
            Self::Funder(kind) => Some(kind.kind()),
            Self::ParliamentMember(_) => None,
        }
    }
}

impl From<&ParliamentMember> for Entity {
    fn from(value: &ParliamentMember) -> Self {
        Self::ParliamentMember(value.clone())
    }
}

impl From<&FunderDetails> for Entity {
    fn from(value: &FunderDetails) -> Self {
        Self::Funder(value.clone())
    }
}

#[derive(Clone, Debug, PartialEq)]
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
    /// Search term
    pub fn term(&self) -> &str {
        self.term.as_str()
    }

    /// Search strictness
    pub fn strictness(&self) -> f32 {
        self.strictness
    }

    /// Max result set
    pub fn max_entries(&self) -> u8 {
        self.max_entries
    }
}

impl EntitySearchRequest {
    /// Creates new instance with [MAX_STRICTNESS]
    pub fn new_strict(term: String, max_entries: u8) -> Result<Self, EntitySearchError> {
        Self::new_with_strictness(term, max_entries, MAX_STRICTNESS)
    }
    /// Creates new instance with variable strictness
    pub fn new_with_strictness(
        term: String,
        max_entries: u8,
        strictness: f32,
    ) -> Result<Self, EntitySearchError> {
        if term.trim().len() < 3 {
            return Err(EntitySearchError::InvalidTerm(term.trim().len() as u8));
        }
        if max_entries < MIN_ENTRIES {
            return Err(EntitySearchError::TooFewEntries);
        }
        if strictness < MIN_STRICTNESS || strictness > MAX_STRICTNESS {
            return Err(EntitySearchError::InvalidStrictness(strictness));
        }
        Ok(Self {
            term,
            max_entries,
            strictness,
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
