//! Ports for complete Lords captures and their separate database projection.

use crate::domain::models::{
    entity_search::EntitySearchRequest,
    lords::{CompleteLordsCapture, LordProfile, LordsError},
    search_similarity::SearchSimilarity,
};

/// Paged register source, always including deleted records.
pub trait LordsSource: Send + Sync {
    /// Fetches the exact bytes for a one-based source page.
    fn page(&self, page: u32) -> impl Future<Output = Result<Vec<u8>, LordsError>> + Send;
}

/// Immutable complete-capture artifacts.
pub trait LordsCaptureStorage: Send + Sync {
    /// Publishes all page bytes and the manifest atomically.
    fn publish(
        &self,
        capture: &CompleteLordsCapture,
    ) -> impl Future<Output = Result<(), LordsError>> + Send;
    /// Verifies the manifest, every digest, and complete pagination.
    fn read(&self) -> impl Future<Output = Result<CompleteLordsCapture, LordsError>> + Send;
}

/// Lords declarations share member identities with Commons declarations.
pub trait LordsRepository: Clone + Send + Sync + 'static {
    /// Upserts members and replaces all their declarations in one transaction.
    fn load_lords(
        &self,
        capture: &CompleteLordsCapture,
    ) -> impl Future<Output = Result<(), LordsError>> + Send;
    /// Primary name matches only, with no effect on Commons phonetic fallback.
    fn search_lords(
        &self,
        request: &EntitySearchRequest,
    ) -> impl Future<Output = Result<Vec<(LordProfile, SearchSimilarity)>, LordsError>> + Send;
}
