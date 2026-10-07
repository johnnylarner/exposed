//! Offline cleaning without database access or identity resolution.

use crate::domain::{
    models::{
        declaration_cleaning::{CleanedDeclarations, DeclarationCleaningSummary},
        entity_ingestion::EntityIngestionError,
    },
    repositories::declaration_cleaning::DeclarationCleaningStorage,
};

/// Separates captured funding occurrences and funder observations.
pub struct DeclarationCleanerService<S> {
    storage: S,
}

impl<S: DeclarationCleaningStorage> DeclarationCleanerService<S> {
    /// Injects storage for raw evidence and completed cleaned tables.
    #[must_use]
    pub const fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Cleans one existing ingestion dataset entirely offline.
    ///
    /// # Errors
    /// Returns source/projection errors, missing input, an existing output, or storage failures.
    pub async fn clean_declarations(
        &self,
    ) -> Result<DeclarationCleaningSummary, EntityIngestionError> {
        let captures = self.storage.read_captured_declarations().await?;
        let cleaned = CleanedDeclarations::from_captures(&captures)?;
        self.storage.publish_cleaned_declarations(&cleaned).await?;
        Ok(cleaned.summary().clone())
    }
}
