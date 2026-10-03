//! Service for ingesting entity search data
//!

use crate::domain::{
    models::entity_ingestion::{EntityIngestionError, EntityIngestionRequest, IngestionKey},
    repositories::{
        entity_ingestion::EntityIngestionStorage, parliament_api::ParliamentApi,
        parliament_member_repository::ParliamentMemberRepo,
    },
};

mod error;
mod interface;

pub use interface::EntityFetcherService;
pub use interface::EntityIngesterService;

/// Allows users to search the databse using free text.
#[derive(Clone)]
pub struct FetcherService<PS, PA> {
    pipeline_storage: PS,
    parliament_api: PA,
}

impl<PS, PA> FetcherService<PS, PA> {
    /// Creates a new instance
    pub const fn new(pipeline_storage: PS, parliament_api: PA) -> Self {
        Self {
            pipeline_storage,
            parliament_api,
        }
    }
}

impl<PS, PA> EntityFetcherService for FetcherService<PS, PA>
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

/// Allows users to search the databse using free text.
#[derive(Clone)]
pub struct IngesterService<PS, PR> {
    pipeline_storage: PS,
    parliament_repo: PR,
}

impl<PS, PR> IngesterService<PS, PR> {
    /// Creates a new instance
    pub const fn new(pipeline_storage: PS, parliament_repo: PR) -> Self {
        Self {
            pipeline_storage,
            parliament_repo,
        }
    }
}

impl<PS, PR> EntityIngesterService for IngesterService<PS, PR>
where
    PS: EntityIngestionStorage,
    PR: ParliamentMemberRepo,
{
    async fn load_members(&self) -> Result<(), EntityIngestionError> {
        let members = self.pipeline_storage.read_raw_members().await?;
        self.parliament_repo.upsert_members(&members).await?;
        Ok(())
    }
}
