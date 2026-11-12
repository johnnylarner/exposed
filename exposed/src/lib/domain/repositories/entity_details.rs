//! Consistent reads for member and funder detail views.

use crate::domain::models::{
    entity_details::{FunderFunding, MemberHistory},
    funder::FunderId,
    lords::LordRegister,
    parliament_member::MemberId,
};
use thiserror::Error;

/// Stored entity detail reads.
pub trait EntityDetailsRepo: Clone + Send + Sync + 'static {
    /// Complete Lords register, independent of Commons declarations.
    fn lord(
        &self,
        id: MemberId,
    ) -> impl Future<Output = Result<Option<LordRegister>, EntityDetailsRepoError>> + Send;
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
