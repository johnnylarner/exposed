//! Capture declaration evidence using stored member identities and domain-owned ports.

use tokio::task::JoinSet;

use crate::domain::{
    models::entity_ingestion::{EntityIngestionError, IngestionKey},
    repositories::{
        entity_ingestion::EntityIngestionStorage, parliament_api::ParliamentApi,
        parliament_member_repository::ParliamentMemberRepo,
    },
};

const MAX_CONCURRENT_MEMBERS: usize = 10;

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
    /// Runs up to ten member tasks concurrently, including their storage writes.
    /// On failure, cancels and awaits the remaining tasks before returning.
    ///
    /// # Errors
    /// Returns errors from the source or storage, task failures, or when no members are stored.
    pub async fn fetch_declarations(&self) -> Result<IngestionKey, EntityIngestionError> {
        let members = self.member_repo.get_stored_member_ids().await?;
        if members.is_empty() {
            return Err(EntityIngestionError::DataError(
                "no stored members; load members with `data members load` before fetching declarations".into(),
            ));
        }
        let mut members = members.into_iter();
        let mut tasks = JoinSet::new();
        loop {
            while tasks.len() < MAX_CONCURRENT_MEMBERS {
                let Some(member) = members.next() else {
                    break;
                };
                let api = self.parliament_api.clone();
                let storage = self.storage.clone();
                tasks.spawn(async move {
                    let declarations = api.get_declarations(member).await?;
                    storage
                        .write_raw_declarations(member, &declarations)
                        .await?;
                    Ok::<(), EntityIngestionError>(())
                });
            }
            let Some(result) = tasks.join_next().await else {
                break;
            };
            let result = result.unwrap_or_else(|error| {
                Err(EntityIngestionError::UnexpectedError(format!(
                    "declaration capture task failed: {error}"
                )))
            });
            if let Err(error) = result {
                tasks.shutdown().await;
                return Err(error);
            }
        }
        Ok(self.storage.ingestion_key())
    }
}

#[cfg(test)]
mod tests;
