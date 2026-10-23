//! Bulk current-term fact reads.
use crate::domain::models::rankings::RankingDeclaration;
use thiserror::Error;
/// Rankings persistence boundary.
pub trait RankingsRepo: Clone + Send + Sync + 'static {
    /// Reads each declaration and funding occurrence once in one snapshot.
    fn ranking_declarations(
        &self,
        start: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<Vec<RankingDeclaration>, RankingsError>> + Send;
}
/// Private database and stored-data diagnostics.
#[derive(Debug, Error)]
#[error("rankings read failed: {0}")]
pub struct RankingsError(pub String);
