//! Consistent reads for member and funder detail views.

use crate::domain::models::{
    entity_details::{FunderFunding, MemberDetails},
    funder::FunderId,
    parliament_member::MemberId,
};
use thiserror::Error;

/// Stored entity detail reads.
pub trait EntityDetailsRepo: Clone + Send + Sync + 'static {
    /// Latest declarations for a member, or absent identity.
    fn member(
        &self,
        id: MemberId,
        recent_limit: usize,
    ) -> impl Future<Output = Result<Option<MemberDetails>, EntityDetailsRepoError>> + Send;
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
