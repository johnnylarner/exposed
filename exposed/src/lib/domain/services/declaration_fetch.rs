//! Capture declaration evidence using stored member identities and domain-owned ports.

use crate::domain::{
    models::entity_ingestion::{EntityIngestionError, IngestionKey},
    repositories::{
        entity_ingestion::EntityIngestionStorage, parliament_api::ParliamentApi,
        parliament_member_repository::ParliamentMemberRepo,
    },
};

/// Fetches and saves declarations for each stored member.
#[derive(Clone)]
pub struct DeclarationFetcherService<PR, PA, PS> {
    member_repo: PR,
    parliament_api: PA,
    storage: PS,
}

impl<PR, PA, PS> DeclarationFetcherService<PR, PA, PS> {
    /// Injects the member repository, source, and ingestion storage.
    #[must_use]
    pub const fn new(member_repo: PR, parliament_api: PA, storage: PS) -> Self {
        Self {
            member_repo,
            parliament_api,
            storage,
        }
    }
}

impl<PR, PA, PS> DeclarationFetcherService<PR, PA, PS>
where
    PR: ParliamentMemberRepo,
    PA: ParliamentApi,
    PS: EntityIngestionStorage,
{
    /// Captures a complete dataset for the cohort read at the start of the run.
    ///
    /// # Errors
    /// Returns errors from the source or storage, or when no members are stored.
    pub async fn fetch_declarations(&self) -> Result<IngestionKey, EntityIngestionError> {
        let members = self.member_repo.get_stored_member_ids().await?;
        if members.is_empty() {
            return Err(EntityIngestionError::DataError(
                "no stored members; load members with `data members load` before fetching declarations".into(),
            ));
        }
        for member in members {
            let declarations = self.parliament_api.get_declarations(member).await?;
            self.storage
                .write_raw_declarations(member, &declarations)
                .await?;
        }
        Ok(self.storage.ingestion_key())
    }
}
