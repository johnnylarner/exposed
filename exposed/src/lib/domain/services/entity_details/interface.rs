use super::EntityDetailsError;
use crate::domain::models::entity_details::{
    FunderDetails, FunderDetailsRequest, MemberDetails, MemberDetailsRequest,
};

/// Entity detail operations with fixed presentation limits.
pub trait EntityDetailsService: Clone + Send + Sync + 'static {
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
