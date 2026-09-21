//! Possible entity search errors

use thiserror::Error;

use crate::domain::repositories::{
    funder_repository::FunderRepoError, parliament_member_repository::ParliamentMemberRepoError,
};

/// Errors related to entity search
#[derive(Error, Debug)]
pub enum EntitySearchError {
    /// Results may be returned by the configured
    /// relevance threshold is not met
    #[error("no relevant entities found for {0}")]
    NoRelevantEntities(String),

    /// Unexpected runtime errors, usually from the database
    #[error("unexpected error occurred: {0}")]
    UnexpectedError(String),
}

impl From<ParliamentMemberRepoError> for EntitySearchError {
    fn from(value: ParliamentMemberRepoError) -> Self {
        match value {
            ParliamentMemberRepoError::DatabaseError(err) => {
                EntitySearchError::UnexpectedError(err)
            }
        }
    }
}

impl From<FunderRepoError> for EntitySearchError {
    fn from(value: FunderRepoError) -> Self {
        match value {
            FunderRepoError::DatabaseError(err) => EntitySearchError::UnexpectedError(err),
        }
    }
}
