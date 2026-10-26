//! Monthly activity persistence boundary.
use crate::domain::models::updates::{RegistrationMonth, UpdateDeclaration};
use thiserror::Error;
/// Reads nonfinancial declaration facts in one snapshot.
pub trait UpdatesRepo: Clone + Send + Sync + 'static {
    /// Reads all registrations within the supplied month.
    fn declarations(
        &self,
        month: &RegistrationMonth,
    ) -> impl Future<Output = Result<Vec<UpdateDeclaration>, UpdatesError>> + Send;
}
/// Stored-data and database diagnostics.
#[derive(Debug, Error)]
#[error("updates read failed: {0}")]
pub struct UpdatesError(pub String);
