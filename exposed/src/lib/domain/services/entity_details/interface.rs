use super::EntityDetailsError;
use crate::domain::models::{
    entity_details::{FunderDetails, MemberDetails},
    funder::FunderId,
    parliament_member::MemberId,
};

/// Entity detail operations with fixed presentation limits.
pub trait EntityDetailsService: Clone + Send + Sync + 'static {
    /// Member identity and recent declarations.
    fn member(
        &self,
        id: MemberId,
    ) -> impl Future<Output = Result<MemberDetails, EntityDetailsError>> + Send;
    /// Exact currency totals and recipient rankings.
    fn funder(
        &self,
        id: FunderId,
    ) -> impl Future<Output = Result<FunderDetails, EntityDetailsError>> + Send;
}
