//! Ports for complete declaration capture and atomic publication.
use crate::domain::models::{
    declaration_ingestion::DeclarationId,
    declaration_loading::DeclarationLoad,
    declaration_sync::{
        CompleteDeclarationSnapshot, DeclarationSyncHead, DiscoveryReceipt, DiscoveryWindow,
        OwnedDeclaration, PreparedPublication,
    },
    entity_ingestion::{EntityIngestionError, IngestionKey},
    parliament_member::MemberId,
};
use std::{collections::BTreeMap, path::Path};

/// Declaration-only Parliament acquisition.
pub trait DeclarationSyncSource: Send + Sync {
    /// Exact source policy recorded in each manifest.
    fn policy(&self) -> String;
    /// Discovers IDs. Filtered payloads are never complete replacement records.
    fn discover(
        &self,
        window: &DiscoveryWindow,
    ) -> impl Future<
        Output = Result<
            (BTreeMap<DeclarationId, MemberId>, Vec<DiscoveryReceipt>),
            EntityIngestionError,
        >,
    > + Send;
    /// Hydrates one ID without publication/update filters.
    fn hydrate(
        &self,
        id: DeclarationId,
    ) -> impl Future<
        Output = Result<(Option<OwnedDeclaration>, Vec<DiscoveryReceipt>), EntityIngestionError>,
    > + Send;
    /// Complete declaration history for an explicit member, without member metadata.
    fn capture_member(
        &self,
        member: MemberId,
    ) -> impl Future<
        Output = Result<(Vec<OwnedDeclaration>, Vec<DiscoveryReceipt>), EntityIngestionError>,
    > + Send;
}

/// Database ownership and publication boundary.
pub trait DeclarationSyncRepository: Send + Sync {
    /// Reads the sole committed head.
    fn sync_head(
        &self,
    ) -> impl Future<Output = Result<Option<DeclarationSyncHead>, EntityIngestionError>> + Send;
    /// Publishes projection and coverage together. A missing load means content is unchanged.
    fn publish_sync(
        &self,
        publication: &PreparedPublication,
        load: Option<&DeclarationLoad>,
    ) -> impl Future<Output = Result<bool, EntityIngestionError>> + Send;
}

/// Durable immutable capture storage and existing interpretation pipeline.
pub trait DeclarationSyncStorage: Send + Sync {
    /// Loads and checks committed evidence and all raw artifact digests.
    ///
    /// # Errors
    /// Rejects missing, incomplete, or modified evidence.
    fn read_snapshot(
        &self,
        path: &Path,
        digest: Option<&str>,
    ) -> Result<CompleteDeclarationSnapshot, EntityIngestionError>;
    /// Publishes raw partitions before sealing the complete capture manifest.
    fn seal_snapshot(
        &self,
        snapshot: CompleteDeclarationSnapshot,
    ) -> impl Future<Output = Result<PreparedPublication, EntityIngestionError>> + Send;
    /// Opens an explicitly resumed run and its checked publication request.
    ///
    /// # Errors
    /// Rejects missing, incomplete, or modified sealed evidence.
    fn open_sealed_run(
        &self,
        run: &IngestionKey,
    ) -> Result<(CompleteDeclarationSnapshot, PreparedPublication), EntityIngestionError>;
    /// Reuses validated completed stage outputs, or runs full cleaning and resolution.
    fn prepare_load(
        &self,
        run: &IngestionKey,
    ) -> impl Future<Output = Result<DeclarationLoad, EntityIngestionError>> + Send;
}
