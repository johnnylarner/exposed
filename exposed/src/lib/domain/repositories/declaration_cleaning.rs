//! Offline declaration evidence and complete cleaned-table publication.

use super::entity_ingestion::EntitySearchPipelineError;
use crate::domain::models::declaration_cleaning::{
    CapturedMemberDeclarations, CleanedDeclarations,
};

/// Storage boundary for offline declaration cleaning.
pub trait DeclarationCleaningStorage: Clone + Send + Sync + 'static {
    /// Replays raw source evidence and checks it against the saved projections.
    fn read_captured_declarations(
        &self,
    ) -> impl Future<Output = Result<Vec<CapturedMemberDeclarations>, EntitySearchPipelineError>> + Send;

    /// Publishes both completed tables together, refusing to overwrite an existing result.
    fn publish_cleaned_declarations(
        &self,
        cleaned: &CleanedDeclarations,
    ) -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
}
