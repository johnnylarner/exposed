//! Contains entity ingestion models

use std::{fmt::Display, str::FromStr};

// use std::str::FromStr;
//
use strum::EnumString;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::repositories::{
    entity_ingestion::EntitySearchPipelineError, parliament_api::ParliamentApiError,
};

/// Data required to search entities
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityIngestionRequest;

/// Key used for ingestion
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestionKey(Uuid);

impl Display for IngestionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.to_string().as_str())
    }
}

impl FromStr for IngestionKey {
    type Err = EntityIngestionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id =
            Uuid::from_str(s).map_err(|e| EntityIngestionError::InvalidStage(e.to_string()))?;
        Ok(Self(id))
    }
}

impl IngestionKey {
    /// Creates a new key
    #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Underlying UUID
    #[must_use]
    pub const fn uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for IngestionKey {
    fn default() -> Self {
        Self::new(Uuid::now_v7())
    }
}

/// From what stage to run in the Member pipeline
#[derive(Clone, Debug, PartialEq, Eq, EnumString)]
#[allow(missing_docs)]
#[strum(ascii_case_insensitive)]
pub enum MemberIngestionStage {
    Fetch,
    Load,
}
/// Supported acquisition stage for declarations.
#[derive(Clone, Debug, PartialEq, Eq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum DeclarationIngestionStage {
    /// Capture source evidence for later offline processing.
    Fetch,
    /// Split captured funding occurrences and funder observations offline.
    Clean,
    /// Resolve cleaned funder observations and attribute each occurrence.
    Resolve,
}

/// Errors related to entity search
#[derive(Error, Debug)]
pub enum EntityIngestionError {
    /// Errors related to the parliament API
    #[error("error when retrieving data from the parliament API: {0}")]
    ApiError(#[from] ParliamentApiError),

    /// Generic error related to entity parsing, cleaning and resolution
    #[error("error in ingestion pipeline: {0}")]
    DataError(String),

    /// Failure reading or writing ingestion storage.
    #[error("ingestion storage failed: {0}")]
    IoError(#[from] EntitySearchPipelineError),

    /// Invalid stage
    #[error("invalid stage provided: {0}")]
    InvalidStage(String),

    /// Invalid stage
    #[error("invalid ingestion ID: {0}")]
    UnvalidIngestionId(String),

    /// Generic error
    #[error("unexpected error occured: {0}")]
    UnexpectedError(String),
}
