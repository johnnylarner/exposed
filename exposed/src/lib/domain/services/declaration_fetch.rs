//! Capture declaration evidence using stored member identities and domain-owned ports.

use crate::domain::{
    models::{
        declaration_ingestion::{
            CapturedDeclaration, DeclarationCaptureOutcome, DeclarationMemberOutput,
        },
        entity_ingestion::EntityIngestionError,
    },
    repositories::{
        entity_ingestion::EntityIngestionStorage, parliament_api::ParliamentApi,
        parliament_member_repository::ParliamentMemberRepo,
    },
};

/// Acquires each member sequentially and publishes completion after all writes succeed.
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
    /// Empty cohorts, existing output, source and storage failures
    /// prevent publication of a completion manifest.
    pub async fn fetch_declarations(
        &self,
    ) -> Result<DeclarationCaptureOutcome, EntityIngestionError> {
        let members = self.member_repo.get_stored_members().await?;
        if members.is_empty() {
            return Err(EntityIngestionError::DataError(
                "no stored members; load members with `data members load` before fetching declarations".into(),
            ));
        }
        self.storage.begin_declarations().await?;
        let mut outputs = Vec::with_capacity(members.len());
        for member in members {
            let declarations = self.parliament_api.get_declarations(member).await?;
            self.storage
                .write_raw_declarations(member, &declarations)
                .await?;
            outputs.push(DeclarationMemberOutput::new(
                member,
                declarations.len(),
                declarations
                    .iter()
                    .map(CapturedDeclaration::row_count)
                    .sum(),
            ));
        }
        let location = self.storage.complete_declarations(&outputs).await?;
        Ok(DeclarationCaptureOutcome::new(
            self.storage.ingestion_key(),
            location,
            outputs,
        ))
    }
}
