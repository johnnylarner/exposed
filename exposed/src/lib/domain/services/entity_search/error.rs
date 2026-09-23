//! Possible entity search errors

use crate::domain::{
    models::entity_search::EntitySearchError,
    repositories::{
        funder_repository::FunderRepoError, parliament_member_repository::ParliamentMemberRepoError,
    },
};

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
