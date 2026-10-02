//! Service for ingesting entity search data
//!

use crate::domain::{
    models::entity_ingestion::{EntityIngestionError, EntityIngestionRequest, IngestionKey},
    repositories::{
        entity_ingestion_pipline::EntityIngestionStorage, parliament_api::ParliamentApi,
    },
};

mod interface;

pub use interface::EntityFetcherService;

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

impl<PS, PA> EntityFetcherService for Service<PS, PA>
where
    PS: EntityIngestionStorage,
    PA: ParliamentApi,
{
    async fn fetch_members(
        &self,
        _req: &EntityIngestionRequest,
    ) -> Result<IngestionKey, EntityIngestionError> {
        let members = self.parliament_api.get_sitting_members().await?;
        self.pipeline_storage.write_raw_members(&members).await?;
        Ok(self.pipeline_storage.ingestion_key())
    }
}
