//! Service for ingesting entity search data
//!

use crate::domain::{
    models::entity_ingestion::{
        DeclarationIngestionStage, EntityIngestionError, EntityIngestionRequest,
        EntityIngestionTarget,
    },
    repositories::{
        entity_ingestion_pipline::EntityIngestionStorage, parliament_api::ParliamentApi,
    },
    services::entity_ingestion::interface::EntitySearchIngestionService,
};

mod interface;

/// Allows users to search the databse using free text.
#[derive(Clone)]
pub struct Service<PS, PA> {
    pipeline_storage: PS,
    parliament_api: PA,
}

impl<PS, PA> Service<PS, PA> {
    /// Creates a new instance
    pub const fn new(pipeline_storage: PS, parliament_api: PA) -> Self {
        Self {
            pipeline_storage,
            parliament_api,
        }
    }
}

impl<PS, PA> EntitySearchIngestionService for Service<PS, PA>
where
    PS: EntityIngestionStorage,
    PA: ParliamentApi,
{
    /// Returns text search results for MPs and Funder entities.
    ///
    /// This function expects the repositories to return their results
    /// in order where a higher score means a higher similarity.
    async fn run_ingestion(
        &self,
        req: &EntityIngestionRequest,
    ) -> Result<(), EntityIngestionError> {
        match req.target() {
            EntityIngestionTarget::Members => self.ingest_members().await,
            EntityIngestionTarget::Declaration(stage) => match stage {
                DeclarationIngestionStage::Import => self.ingest_declarations().await,
                DeclarationIngestionStage::Clean => self.clean_funders().await,
                DeclarationIngestionStage::Resolve => self.resolve_funders().await,
            },
        }
    }
}

impl<PS, PA> Service<PS, PA>
where
    PA: ParliamentApi,
{
    async fn fetch_members(&self) -> Result<(), EntityIngestionError> {
        self.parliament_api.get_sitting_members().await?;
        Ok(())
    }

    async fn fetch_declarations(&self) -> Result<(), EntityIngestionError> {
        self.parliament_api
            .get_declarations_for_sitting_members()
            .await?;
        Ok(())
    }
}

impl<PS, PA> Service<PS, PA>
where
    PS: EntityIngestionStorage,
    PA: ParliamentApi,
{
    async fn ingest_members(&self) -> Result<(), EntityIngestionError> {
        self.fetch_members().await?;
        self.pipeline_storage.write_raw_data().await?;
        Ok(())
    }

    async fn ingest_declarations(&self) -> Result<(), EntityIngestionError> {
        self.fetch_declarations().await?;
        self.pipeline_storage.write_raw_data().await?;
        Ok(())
    }
}

impl<PS, PA> Service<PS, PA>
where
    PS: EntityIngestionStorage,
{
    async fn clean_funders(&self) -> Result<(), EntityIngestionError> {
        self.pipeline_storage.read_raw_data().await?;
        self.pipeline_storage.write_cleaned_data().await?;
        Ok(())
    }

    async fn resolve_funders(&self) -> Result<(), EntityIngestionError> {
        self.pipeline_storage.read_cleaned_data().await?;
        self.pipeline_storage.write_resolved_data().await?;
        Ok(())
    }
}
