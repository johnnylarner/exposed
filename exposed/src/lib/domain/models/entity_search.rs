use strum::{AsRefStr, EnumString};
use thiserror::Error;

const MAX_STRICTNESS: f32 = 1_f32;
const MIN_STRICTNESS: f32 = 0_f32;

#[derive(Clone, Debug, PartialEq)]
/// Search entity result
pub struct Entity {
    name: EntityName,
    kind: EntityKind,
}

impl Entity {
    /// Creates a new entity
    pub fn new(name: String, kind: EntityKind) -> Self {
        Self {
            name: EntityName(name),
            kind,
        }
    }

    /// Entity name
    pub fn name(&self) -> &str {
        &self.name.0
    }

    /// Entity kind
    pub fn kind(&self) -> &str {
        self.kind.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Search entity name
pub struct EntityName(String);

#[derive(Clone, Debug, EnumString, AsRefStr, PartialEq)]
/// Search entity kind
pub enum EntityKind {
    /// Company entity
    Company,
    /// Member entitiy
    ParliamentMember,
}

#[derive(Clone, Debug, PartialEq)]
/// Data required to search entities
pub struct EntitySearchRequest {
    term: String,
    strictness: f32,
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
}

impl EntitySearchRequest {
    /// Creates new instance with [MAX_STRICTNESS]
    pub fn new_strict(term: String) -> Result<Self, EntitySearchError> {
        Self::new_with_strictness(term, MAX_STRICTNESS)
    }
    /// Creates new instance with variable strictness
    pub fn new_with_strictness(term: String, strictness: f32) -> Result<Self, EntitySearchError> {
        if term.trim().len() < 3 {
            return Err(EntitySearchError::InvalidTerm(term.trim().len() as u8));
        }
        if strictness < MIN_STRICTNESS || strictness > MAX_STRICTNESS {
            return Err(EntitySearchError::InvalidStrictness(strictness));
        }
        Ok(Self { term, strictness })
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
    #[error("strictness must be between {MIN_STRICTNESS} and {MAX_STRICTNESS}; get {0}")]
    InvalidStrictness(f32),
}
