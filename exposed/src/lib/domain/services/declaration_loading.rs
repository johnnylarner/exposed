//! Coordinates the checked declaration dataset with its atomic database import.

use crate::domain::{
    models::{declaration_loading::DeclarationLoadSummary, entity_ingestion::EntityIngestionError},
    repositories::declaration_loading::{DeclarationLoadRepository, DeclarationLoadStorage},
};

/// Loads resolved declarations into the current database projection.
pub struct DeclarationLoaderService<S, R> {
    storage: S,
    repository: R,
}

impl<S, R> DeclarationLoaderService<S, R> {
    /// Connects the resolved-run source to the database repository.
    #[must_use]
    pub const fn new(storage: S, repository: R) -> Self {
        Self {
            storage,
            repository,
        }
    }
}

impl<S: DeclarationLoadStorage, R: DeclarationLoadRepository> DeclarationLoaderService<S, R> {
    /// Reads and atomically loads one resolved declaration run.
    ///
    /// # Errors
    /// Returns source validation, stale-run, conflict, or database errors.
    pub async fn load_declarations(&self) -> Result<DeclarationLoadSummary, EntityIngestionError> {
        let load = self.storage.read_declaration_load().await?;
        self.repository.load_declarations(&load).await
    }
}
