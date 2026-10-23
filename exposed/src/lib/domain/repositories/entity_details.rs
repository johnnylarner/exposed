//! Consistent reads for member and funder detail views.

use crate::domain::models::{
    entity_details::{FunderFunding, MemberHistory},
    funder::FunderId,
    parliament_member::MemberId,
};
use thiserror::Error;

/// Stored entity detail reads.
pub trait EntityDetailsRepo: Clone + Send + Sync + 'static {
    /// All current Commons histories from one consistent snapshot.
    fn current_commons(
        &self,
    ) -> impl Future<Output = Result<Vec<MemberHistory>, EntityDetailsRepoError>> + Send;
    /// All declarations for a member, or absent identity.
    fn member(
        &self,
        id: MemberId,
    ) -> impl Future<Output = Result<Option<MemberHistory>, EntityDetailsRepoError>> + Send;
    /// Recipient/currency aggregates for a funder, or absent identity.
    fn funder_funding(
        &self,
        id: FunderId,
    ) -> impl Future<Output = Result<Option<FunderFunding>, EntityDetailsRepoError>> + Send;
}

/// Read failures retain private diagnostics.
#[derive(Debug, Error)]
pub enum EntityDetailsRepoError {
    /// Unexpected database or stored-data error.
    #[error("entity details read failed: {0}")]
    DatabaseError(String),
}
