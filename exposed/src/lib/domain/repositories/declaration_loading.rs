use crate::domain::models::{
    declaration_loading::{DeclarationLoad, DeclarationLoadSummary},
    entity_ingestion::EntityIngestionError,
};

/// Reads and validates all files required for one resolved run.
pub trait DeclarationLoadStorage: Clone + Send + Sync + 'static {
    /// Returns a checked projection without modifying the source files.
    fn read_declaration_load(
        &self,
    ) -> impl Future<Output = Result<DeclarationLoad, EntityIngestionError>> + Send;
}

/// Publishes one checked run as the current searchable declaration data.
pub trait DeclarationLoadRepository: Clone + Send + Sync + 'static {
    /// Commits the run atomically or returns its exact-retry summary.
    fn load_declarations(
        &self,
        load: &DeclarationLoad,
    ) -> impl Future<Output = Result<DeclarationLoadSummary, EntityIngestionError>> + Send;
}
