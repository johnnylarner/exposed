//! Declaration-ID synchronization over complete retained evidence.
use std::collections::{BTreeMap, BTreeSet};

use chrono::{Days, NaiveDate, Utc};
use sha2::{Digest, Sha256};

use crate::domain::{
    models::{
        declaration_ingestion::DeclarationId,
        declaration_sync::{
            CompleteDeclarationSnapshot, DeclarationSyncMode, DiscoveryWindow, OwnedDeclaration,
            PreparedPublication,
        },
        entity_ingestion::{EntityIngestionError, IngestionKey},
        parliament_member::MemberId,
    },
    repositories::{
        declaration_sync::{
            DeclarationSyncRepository, DeclarationSyncSource, DeclarationSyncStorage,
        },
        parliament_member_repository::ParliamentMemberRepo,
    },
};

struct AcquiredDeclarations {
    declarations: BTreeMap<DeclarationId, OwnedDeclaration>,
    receipts: Vec<crate::domain::models::declaration_sync::DiscoveryReceipt>,
    exclusions: BTreeMap<DeclarationId, MemberId>,
    omissions: Vec<DeclarationId>,
}

/// Runtime inputs independent of member metadata maintenance.
pub struct DeclarationSyncOptions {
    /// Source UTC cutoff. Defaults at the CLI boundary to UTC today minus indexing lag.
    pub cutoff: NaiveDate,
    /// Number of previous coverage dates to rediscover.
    pub overlap_days: u64,
}

/// Coordinates source acquisition, immutable evidence, global interpretation, and publication.
pub struct DeclarationSyncService<R, S, A> {
    repository: R,
    storage: S,
    source: A,
}

impl<R, S, A> DeclarationSyncService<R, S, A>
where
    R: DeclarationSyncRepository + ParliamentMemberRepo,
    S: DeclarationSyncStorage,
    A: DeclarationSyncSource,
{
    /// Injects the database, artifact store, and declaration-only source.
    pub const fn new(repository: R, storage: S, source: A) -> Self {
        Self {
            repository,
            storage,
            source,
        }
    }

    /// Captures complete evidence and atomically publishes it.
    ///
    /// # Errors
    /// Rejects missing baselines, unknown ownership, incomplete source requests, changed evidence, or stale publication.
    pub async fn synchronize(
        &self,
        mode: DeclarationSyncMode,
        options: &DeclarationSyncOptions,
    ) -> Result<IngestionKey, EntityIngestionError> {
        if options.cutoff > Utc::now().date_naive() {
            return Err(invalid(
                "cutoff cannot be after the current UTC source date",
            ));
        }
        let head = self.repository.sync_head().await?;
        let base = head
            .as_ref()
            .map(|head| {
                self.storage
                    .read_snapshot(&head.manifest_path, Some(&head.manifest_sha256))
            })
            .transpose()?;
        if matches!(mode, DeclarationSyncMode::Daily) && base.is_none() {
            return Err(invalid(
                "daily synchronization requires `declarations reconcile` to initialize complete evidence",
            ));
        }
        if matches!(mode, DeclarationSyncMode::Daily)
            && base
                .as_ref()
                .is_some_and(|base| base.source != self.source.policy())
        {
            return Err(invalid(
                "source policy changed; explicit reconciliation is required",
            ));
        }
        let stored = self
            .repository
            .get_stored_member_ids()
            .await?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let scope = capture_scope(mode, base.as_ref(), &stored)?;
        let window = discovery_window(base.as_ref(), options)?;
        let declarations = base
            .as_ref()
            .map_or_else(BTreeMap::new, |base| base.declarations.clone());
        if declarations
            .values()
            .any(|record| !scope.contains(&record.member))
        {
            return Err(invalid(
                "reconciliation cannot drop members from retained declaration scope",
            ));
        }
        let mut acquired = self.acquire(mode, &scope, &window, declarations).await?;
        self.hydrate_parents(&mut acquired).await?;
        let AcquiredDeclarations {
            declarations,
            receipts,
            exclusions,
            omissions,
        } = acquired;
        let content_sha256 = source_content_digest(&declarations)?;
        let content_changed = base
            .as_ref()
            .is_none_or(|base| base.content_sha256 != content_sha256);
        let run = IngestionKey::default();
        let snapshot = CompleteDeclarationSnapshot {
            schema_version: 1,
            run: run.uuid(),
            expected_revision: head.map_or(0, |head| head.revision),
            mode,
            source: self.source.policy(),
            window,
            pending_members: stored
                .iter()
                .filter(|member| !scope.contains(member))
                .copied()
                .collect(),
            scope_sha256: sha256_digest(&serde_json::to_vec(&scope).map_err(failure)?),
            scope,
            declarations,
            exclusions,
            omissions,
            receipts,
            artifacts: BTreeMap::new(),
            content_sha256,
            content_changed,
            captured_at: Utc::now(),
        };
        let publication = self.storage.seal_snapshot(snapshot).await?;
        self.publish(&run, &publication, content_changed).await.map_err(|error| invalid(&format!("sealed declaration run {run}; recover with declarations sync --resume {run}: {error}")))?;
        Ok(run)
    }

    async fn acquire(
        &self,
        mode: DeclarationSyncMode,
        scope: &[MemberId],
        window: &DiscoveryWindow,
        mut declarations: BTreeMap<DeclarationId, OwnedDeclaration>,
    ) -> Result<AcquiredDeclarations, EntityIngestionError> {
        let mut receipts = Vec::new();
        let mut exclusions = BTreeMap::new();
        let mut omissions = Vec::new();
        match mode {
            DeclarationSyncMode::Daily => {
                let (discovered, pages) = self.source.discover(window).await?;
                receipts.extend(pages);
                for (id, owner) in discovered {
                    if declarations
                        .get(&id)
                        .is_some_and(|previous| previous.member != owner)
                    {
                        return Err(invalid(
                            "discovery registrant disagrees with retained declaration ownership",
                        ));
                    }
                    if !scope.contains(&owner) {
                        exclusions.insert(id, owner);
                        continue;
                    }
                    let (record, pages) = self.source.hydrate(id).await?;
                    receipts.extend(pages);
                    match record {
                        Some(record) => {
                            if record.member != owner || record.declaration.id() != id {
                                return Err(invalid(
                                    "hydration ownership or declaration identity disagrees with discovery",
                                ));
                            }
                            merge_record(&mut declarations, record)?;
                        }
                        None => omissions.push(id),
                    }
                }
            }
            DeclarationSyncMode::Reconcile => {
                let mut observed = BTreeSet::new();
                for member in scope {
                    let (records, pages) = self.source.capture_member(*member).await?;
                    receipts.extend(pages);
                    for record in records {
                        if record.member != *member || !observed.insert(record.declaration.id()) {
                            return Err(invalid(
                                "complete capture has mismatched ownership or duplicate declaration IDs",
                            ));
                        }
                        merge_record(&mut declarations, record)?;
                    }
                }
                omissions.extend(
                    declarations
                        .keys()
                        .filter(|id| !observed.contains(id))
                        .copied(),
                );
            }
        }
        Ok(AcquiredDeclarations {
            declarations,
            receipts,
            exclusions,
            omissions,
        })
    }

    async fn hydrate_parents(
        &self,
        acquired: &mut AcquiredDeclarations,
    ) -> Result<(), EntityIngestionError> {
        let declarations = &mut acquired.declarations;
        let receipts = &mut acquired.receipts;
        let mut checked_parents = BTreeSet::new();
        loop {
            let mut missing = BTreeMap::<_, BTreeSet<_>>::new();
            for record in declarations.values() {
                if let Some(id) = record.declaration.parent_id()
                    && !declarations.contains_key(&id)
                    && !checked_parents.contains(&id)
                {
                    missing.entry(id).or_default().insert(record.member);
                }
            }
            if missing.is_empty() {
                break;
            }
            for (id, members) in missing {
                checked_parents.insert(id);
                let (record, pages) = self.source.hydrate(id).await?;
                receipts.extend(pages);
                match record {
                    Some(record) if members.contains(&record.member) => {
                        merge_record(declarations, record)?;
                    }
                    Some(_) | None => {}
                }
            }
        }
        Ok(())
    }

    /// Continues sealed evidence without reacquiring or modifying source records.
    ///
    /// # Errors
    /// Rejects unsealed, changed, or stale evidence.
    pub async fn resume(&self, run: IngestionKey) -> Result<IngestionKey, EntityIngestionError> {
        let (snapshot, publication) = self.storage.open_sealed_run(&run)?;
        if snapshot.run != run.uuid() {
            return Err(invalid("resume manifest belongs to another run"));
        }
        self.publish(&run, &publication, snapshot.content_changed).await.map_err(|error| invalid(&format!("sealed declaration run {run}; recover with declarations sync --resume {run}: {error}")))?;
        Ok(run)
    }

    async fn publish(
        &self,
        run: &IngestionKey,
        publication: &PreparedPublication,
        changed: bool,
    ) -> Result<(), EntityIngestionError> {
        let load = if changed {
            Some(self.storage.prepare_load(run).await?)
        } else {
            None
        };
        self.repository
            .publish_sync(publication, load.as_ref())
            .await?;
        Ok(())
    }
}

fn discovery_window(
    base: Option<&CompleteDeclarationSnapshot>,
    options: &DeclarationSyncOptions,
) -> Result<DiscoveryWindow, EntityIngestionError> {
    let previous_date = base.map_or(options.cutoff, |base| base.window.through);
    if options.cutoff < previous_date {
        return Err(invalid("cutoff cannot move committed coverage backwards"));
    }
    let window = DiscoveryWindow {
        from: previous_date
            .checked_sub_days(Days::new(options.overlap_days))
            .ok_or_else(|| invalid("discovery overlap exceeds date range"))?,
        through: options.cutoff,
    };
    Ok(window)
}

fn capture_scope(
    mode: DeclarationSyncMode,
    base: Option<&CompleteDeclarationSnapshot>,
    stored: &BTreeSet<MemberId>,
) -> Result<Vec<MemberId>, EntityIngestionError> {
    if stored.is_empty() {
        return Err(invalid("load members before capturing declarations"));
    }
    let scope = match (&mode, base) {
        (DeclarationSyncMode::Daily, Some(base)) => {
            if base.scope.iter().any(|member| !stored.contains(member)) {
                return Err(invalid(
                    "a member in the committed declaration scope is no longer stored",
                ));
            }
            base.scope.clone()
        }
        (DeclarationSyncMode::Daily, None) => unreachable!("daily baseline checked above"),
        (DeclarationSyncMode::Reconcile, _) => stored.iter().copied().collect(),
    };
    Ok(scope)
}

fn merge_record(
    records: &mut BTreeMap<
        crate::domain::models::declaration_ingestion::DeclarationId,
        OwnedDeclaration,
    >,
    record: OwnedDeclaration,
) -> Result<(), EntityIngestionError> {
    let id = record.declaration.id();
    if let Some(previous) = records.get(&id) {
        if previous.member != record.member {
            return Err(invalid("source declaration changed registrant"));
        }
        if previous.declaration.source_json() == record.declaration.source_json() {
            return Ok(());
        }
    }
    records.insert(id, record);
    Ok(())
}

pub(crate) fn source_content_digest(
    records: &BTreeMap<
        crate::domain::models::declaration_ingestion::DeclarationId,
        OwnedDeclaration,
    >,
) -> Result<String, EntityIngestionError> {
    let content = records
        .iter()
        .map(|(id, record)| (id, record.member, record.declaration.source_json()))
        .collect::<Vec<_>>();
    Ok(sha256_digest(
        &serde_json::to_vec(&content).map_err(failure)?,
    ))
}
pub(crate) fn sha256_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn invalid(message: &str) -> EntityIngestionError {
    EntityIngestionError::DataError(message.into())
}
fn failure(error: impl std::fmt::Display) -> EntityIngestionError {
    EntityIngestionError::DataError(error.to_string())
}
