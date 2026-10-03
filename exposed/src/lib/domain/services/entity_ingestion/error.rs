//! Possible entity ingestion errors

use crate::domain::{
    models::entity_ingestion::EntityIngestionError,
    repositories::{
        funder_repository::FunderRepoError, parliament_member_repository::ParliamentMemberRepoError,
    },
};

impl From<ParliamentMemberRepoError> for EntityIngestionError {
    fn from(value: ParliamentMemberRepoError) -> Self {
        match value {
            ParliamentMemberRepoError::DatabaseError(err) => Self::UnexpectedError(err),
        }
    }
}

impl From<FunderRepoError> for EntityIngestionError {
    fn from(value: FunderRepoError) -> Self {
        match value {
            FunderRepoError::DatabaseError(err) => Self::UnexpectedError(err),
        }
    }
}
