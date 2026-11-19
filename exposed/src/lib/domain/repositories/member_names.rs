//! Ports for preferred-name capture and atomic refresh.
use crate::domain::models::{
    member_names::{MemberNames, MemberNamesCapture, MemberNamesError},
    membership::Chamber,
    parliament_member::MemberId,
};
use std::collections::BTreeMap;

/// Official raw MNIS responses.
pub trait MemberNamesSource: Send + Sync {
    /// Fetch current members of a chamber.
    fn chamber(
        &self,
        chamber: Chamber,
    ) -> impl Future<Output = Result<String, MemberNamesError>> + Send;
    /// Fetch a historical identity absent from the current bulk responses.
    fn member(&self, id: MemberId)
    -> impl Future<Output = Result<String, MemberNamesError>> + Send;
}
/// Replayable complete source evidence.
pub trait MemberNamesStorage: Send + Sync {
    /// Atomically publishes a complete capture.
    ///
    /// # Errors
    /// Rejects invalid evidence, an existing destination, and storage failures.
    fn publish(&self, capture: &MemberNamesCapture) -> Result<(), MemberNamesError>;
    /// Reads retained evidence without network access.
    ///
    /// # Errors
    /// Rejects unreadable or malformed capture files.
    fn read(&self) -> Result<MemberNamesCapture, MemberNamesError>;
}
/// Existing member rows own all names. This operation never creates identities.
pub trait MemberNamesRepository: Send + Sync {
    /// All identities whose names must be covered.
    fn targets(
        &self,
    ) -> impl Future<Output = Result<Vec<(MemberId, Chamber)>, MemberNamesError>> + Send;
    /// Checks complete coverage and updates names in one transaction.
    fn refresh(
        &self,
        names: &BTreeMap<MemberId, MemberNames>,
    ) -> impl Future<Output = Result<usize, MemberNamesError>> + Send;
}
