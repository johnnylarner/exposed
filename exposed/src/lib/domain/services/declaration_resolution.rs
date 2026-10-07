//! Coordinates checked evidence, statistical scoring, Rust policy, and publication.
use crate::domain::{
    models::{
        declaration_resolution::{DeclarationResolutionSummary, ResolvedDeclarations},
        entity_ingestion::EntityIngestionError,
    },
    repositories::declaration_resolution::{DeclarationResolutionStorage, FunderScorer},
};

/// Offline resolver with an explicitly configured statistical scorer.
pub struct DeclarationResolverService<S, F> {
    storage: S,
    scorer: F,
    candidate_budget: usize,
}
impl<S: DeclarationResolutionStorage, F: FunderScorer> DeclarationResolverService<S, F> {
    /// Supplies storage, scorer and the maximum candidate count.
    #[must_use]
    pub const fn new(storage: S, scorer: F, candidate_budget: usize) -> Self {
        Self {
            storage,
            scorer,
            candidate_budget,
        }
    }
    /// Resolves and publishes one ingestion run.
    ///
    /// # Errors
    /// Fails on incoherent evidence, scorer errors, budget refusal, or publication errors.
    pub async fn resolve_declarations(
        &self,
    ) -> Result<DeclarationResolutionSummary, EntityIngestionError> {
        let input = self.storage.read_resolution_input().await?;
        let scoring = input.scoring_input(self.candidate_budget);
        let scored = self.scorer.score(&scoring).await?;
        let result = ResolvedDeclarations::from_scored(
            &input,
            &scoring,
            scored,
            &self.storage.resolution_run_key(),
        )?;
        self.storage.publish_resolution(&result).await?;
        Ok(result.summary())
    }
}
