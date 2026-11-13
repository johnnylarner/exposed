//! Orchestration for complete capture and transactional import of Lords interests.

use crate::domain::{
    models::{
        entity_ingestion::IngestionKey,
        lords::{CompleteLordsCapture, LordsCaptureBuilder, LordsError, LordsLoadOutcome},
    },
    repositories::lords::{LordsCaptureStorage, LordsRepository, LordsSource},
};

/// Fetches all pages before publishing a replayable capture.
pub struct LordsFetcher<S, F> {
    source: S,
    storage: F,
}

impl<S: LordsSource, F: LordsCaptureStorage> LordsFetcher<S, F> {
    /// Injects the source and immutable artifact store.
    pub const fn new(source: S, storage: F) -> Self {
        Self { source, storage }
    }

    /// Captures a contiguous, complete source register.
    ///
    /// # Errors
    /// Any source or pagination failure leaves no published capture.
    pub async fn fetch(&self, key: IngestionKey) -> Result<CompleteLordsCapture, LordsError> {
        let mut builder = LordsCaptureBuilder::new(key, chrono::Utc::now());
        let mut page = 1_u32;
        while !builder.is_complete() {
            builder.push_page(self.source.page(page).await?)?;
            page = page
                .checked_add(1)
                .ok_or_else(|| LordsError::Capture("page number overflow".into()))?;
        }
        let capture = builder.finish()?;
        self.storage.publish(&capture).await?;
        Ok(capture)
    }
}

/// Validates a retained artifact before the repository opens a transaction.
pub struct LordsLoader<F, R> {
    storage: F,
    repo: R,
}

impl<F: LordsCaptureStorage, R: LordsRepository> LordsLoader<F, R> {
    /// Injects the artifact store and Lords repository.
    pub const fn new(storage: F, repo: R) -> Self {
        Self { storage, repo }
    }

    /// Loads a whole capture or confirms an exact retry.
    ///
    /// # Errors
    /// Rejects incomplete evidence, reused identities, and database failures.
    pub async fn load(&self) -> Result<(LordsLoadOutcome, usize), LordsError> {
        let capture = self.storage.read().await?;
        let outcome = self.repo.load_lords(&capture).await?;
        Ok((outcome, capture.members().len()))
    }
}
