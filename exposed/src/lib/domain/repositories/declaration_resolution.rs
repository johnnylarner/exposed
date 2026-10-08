//! Boundaries for checked cleaned evidence, statistical scores, and complete output.
use super::entity_ingestion::EntitySearchPipelineError;
use crate::domain::models::{
    declaration_resolution::{ResolutionInput, ResolvedDeclarations, ScoredPairs, ScoringInput},
    entity_ingestion::EntityIngestionError,
};

/// Storage for one immutable declaration resolution run.
pub trait DeclarationResolutionStorage: Clone + Send + Sync + 'static {
    /// Reads and checks the cleaned observation and funding tables together.
    fn read_resolution_input(
        &self,
    ) -> impl Future<Output = Result<ResolutionInput, EntityIngestionError>> + Send;
    /// Publishes every completed output together without replacement.
    fn publish_resolution(
        &self,
        result: &ResolvedDeclarations,
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
    /// Namespace for local provisional identities.
    fn resolution_run_key(&self) -> String;
}
/// Statistical scoring port. The implementation must report its actual model/runtime.
pub trait FunderScorer: Send + Sync {
    /// Scores bounded candidate profile pairs without assigning identities.
    fn score(
        &self,
        input: &ScoringInput,
    ) -> impl Future<Output = Result<ScoredPairs, EntityIngestionError>> + Send;
}
