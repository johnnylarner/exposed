use super::EntityDetailsError;
use crate::domain::models::entity_details::{
    FunderDetails, FunderDetailsRequest, MemberDetails, MemberDetailsRequest,
};
use crate::domain::models::{lords::LordRegister, parliament_member::MemberId};

/// Entity detail operations with fixed presentation limits.
pub trait EntityDetailsService: Clone + Send + Sync + 'static {
    /// All captured Lords registered interests, without a reporting-period filter.
    fn lord(
        &self,
        id: MemberId,
    ) -> impl Future<Output = Result<LordRegister, EntityDetailsError>> + Send;
    /// Member identity and recent declarations.
    fn member(
        &self,
        request: MemberDetailsRequest,
    ) -> impl Future<Output = Result<MemberDetails, EntityDetailsError>> + Send;
    /// Exact currency totals and recipient rankings.
    fn funder(
        &self,
        request: FunderDetailsRequest,
    ) -> impl Future<Output = Result<FunderDetails, EntityDetailsError>> + Send;
}
