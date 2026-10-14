use super::{ExposedDataPipeline, ExposedDatabase, WeldrsFunderScorer};
use crate::domain::{
    models::{
        declaration_loading::DeclarationLoad,
        declaration_sync::{CompleteDeclarationSnapshot, DeclarationSyncHead, PreparedPublication},
        entity_ingestion::{DeclarationIngestionStage, EntityIngestionError, IngestionKey},
    },
    repositories::{
        declaration_loading::DeclarationLoadStorage,
        declaration_sync::{DeclarationSyncRepository, DeclarationSyncStorage},
        entity_ingestion::EntityIngestionStorage,
    },
    services::{
        declaration_cleaning::DeclarationCleanerService,
        declaration_resolution::DeclarationResolverService,
        declaration_sync::{sha256_digest, source_content_digest},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// Artifact root and global resolution policy for declaration synchronization.
pub struct DeclarationSnapshotStore {
    root: PathBuf,
    candidate_budget: usize,
}
impl DeclarationSnapshotStore {
    /// Uses an absolute artifact root so committed references survive working-directory changes.
    ///
    /// # Errors
    /// Rejects an invalid root or zero candidate budget.
    pub fn new(root: &Path, candidate_budget: usize) -> Result<Self, EntityIngestionError> {
        if candidate_budget == 0 {
            return Err(error("candidate_budget must be positive"));
        }
        Ok(Self {
            root: std::path::absolute(root).map_err(error)?,
            candidate_budget,
        })
    }
}
impl DeclarationSyncStorage for DeclarationSnapshotStore {
    fn open_sealed_run(
        &self,
        run: &IngestionKey,
    ) -> Result<(CompleteDeclarationSnapshot, PreparedPublication), EntityIngestionError> {
        let path = self.root.join(run.to_string()).join("capture.json");
        let snapshot = self.read_snapshot(&path, None)?;
        let manifest_sha256 =
            fs::read_to_string(path.with_file_name("capture.sha256")).map_err(error)?;
        let publication = PreparedPublication {
            run: snapshot.run,
            expected_revision: snapshot.expected_revision,
            manifest_path: path,
            manifest_sha256,
            coverage_date: snapshot.window.through,
        };
        Ok((snapshot, publication))
    }
    fn read_snapshot(
        &self,
        path: &Path,
        expected_digest: Option<&str>,
    ) -> Result<CompleteDeclarationSnapshot, EntityIngestionError> {
        let bytes = fs::read(path).map_err(error)?;
        let marker = fs::read_to_string(path.with_file_name("capture.sha256")).map_err(error)?;
        if sha256_digest(&bytes) != marker
            || expected_digest.is_some_and(|expected| sha256_digest(&bytes) != expected)
        {
            return Err(error("committed capture manifest digest mismatch"));
        }
        let snapshot: CompleteDeclarationSnapshot =
            serde_json::from_slice(&bytes).map_err(error)?;
        if snapshot.schema_version != 1
            || snapshot.expected_revision < 0
            || snapshot.scope.is_empty()
            || snapshot.window.from > snapshot.window.through
            || snapshot
                .scope
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                != snapshot.scope
            || snapshot.scope_sha256
                != sha256_digest(&serde_json::to_vec(&snapshot.scope).map_err(error)?)
            || snapshot.content_sha256 != source_content_digest(&snapshot.declarations)?
        {
            return Err(error("invalid complete declaration manifest"));
        }
        let directory = path
            .parent()
            .ok_or_else(|| error("manifest has no parent"))?;
        let expected_paths = snapshot
            .scope
            .iter()
            .map(|member| PathBuf::from(format!("raw/declarations/{member}.parquet")))
            .collect::<BTreeSet<_>>();
        if snapshot.artifacts.keys().cloned().collect::<BTreeSet<_>>() != expected_paths {
            return Err(error("manifest does not cover every cohort partition"));
        }
        for (relative, expected) in &snapshot.artifacts {
            if sha256_digest(&fs::read(directory.join(relative)).map_err(error)?) != *expected {
                return Err(error("raw declaration artifact digest mismatch"));
            }
            let partition = super::declaration_cleaning::read_partition(&directory.join(relative))?;
            let expected_records = snapshot
                .declarations
                .values()
                .filter(|record| record.member == partition.member)
                .count();
            if partition.declarations.len() != expected_records {
                return Err(error("partition differs from sealed declaration set"));
            }
            for evidence in partition.declarations {
                let actual = evidence.declaration();
                let expected = snapshot
                    .declarations
                    .get(&actual.id())
                    .ok_or_else(|| error("unexpected declaration in raw partition"))?;
                if expected.member != partition.member
                    || actual.source_json() != expected.declaration.source_json()
                    || actual.fetched_at() != expected.declaration.fetched_at()
                {
                    return Err(error("raw partition disagrees with capture manifest"));
                }
            }
        }
        for (id, record) in &snapshot.declarations {
            if *id != record.declaration.id() || !snapshot.scope.contains(&record.member) {
                return Err(error("manifest contains invalid declaration ownership"));
            }
        }
        Ok(snapshot)
    }
    async fn seal_snapshot(
        &self,
        mut snapshot: CompleteDeclarationSnapshot,
    ) -> Result<PreparedPublication, EntityIngestionError> {
        fs::create_dir_all(&self.root).map_err(error)?;
        let key = IngestionKey::new(snapshot.run);
        let staging_root = self.root.join(format!(".capture-{}", snapshot.run));
        fs::create_dir(&staging_root).map_err(error)?;
        let storage = ExposedDataPipeline::new_with_ingestion_key(&staging_root, key.clone())?;
        let staged_run = staging_root.join(key.to_string());
        for member in &snapshot.scope {
            let records = snapshot
                .declarations
                .values()
                .filter(|record| record.member == *member)
                .map(|record| record.declaration.clone())
                .collect::<Vec<_>>();
            storage.write_raw_declarations(*member, &records).await?;
            let relative = PathBuf::from(format!("raw/declarations/{member}.parquet"));
            snapshot.artifacts.insert(
                relative.clone(),
                sha256_digest(&fs::read(staged_run.join(relative)).map_err(error)?),
            );
        }
        let bytes = serde_json::to_vec_pretty(&snapshot).map_err(error)?;
        fs::write(staged_run.join("capture.json"), &bytes).map_err(error)?;
        fs::write(staged_run.join("capture.sha256"), sha256_digest(&bytes)).map_err(error)?;
        sync_tree(&staged_run)?;
        let destination = self.root.join(key.to_string());
        if destination.exists() {
            return Err(error("capture run already exists"));
        }
        fs::rename(&staged_run, &destination).map_err(error)?;
        fs::File::open(&self.root)
            .and_then(|file| file.sync_all())
            .map_err(error)?;
        fs::remove_dir(staging_root).map_err(error)?;
        let manifest_path = destination.join("capture.json");
        self.read_snapshot(&manifest_path, Some(&sha256_digest(&bytes)))?;
        Ok(PreparedPublication {
            run: snapshot.run,
            expected_revision: snapshot.expected_revision,
            manifest_path,
            manifest_sha256: sha256_digest(&bytes),
            coverage_date: snapshot.window.through,
        })
    }
    async fn prepare_load(
        &self,
        run: &IngestionKey,
    ) -> Result<DeclarationLoad, EntityIngestionError> {
        let directory = self.root.join(run.to_string());
        self.read_snapshot(&directory.join("capture.json"), None)?;
        let storage = ExposedDataPipeline::open_existing_declarations(&self.root, run.clone())?;
        for stage in [
            DeclarationIngestionStage::Clean,
            DeclarationIngestionStage::Resolve,
        ] {
            let name = match stage {
                DeclarationIngestionStage::Clean => "cleaned",
                DeclarationIngestionStage::Resolve => "resolved",
                DeclarationIngestionStage::Fetch
                | DeclarationIngestionStage::Load
                | DeclarationIngestionStage::Sync
                | DeclarationIngestionStage::Reconcile => unreachable!("derived stages only"),
            };
            let output = directory.join(name).join("declarations");
            let receipt = directory.join(format!("{name}-sha256.json"));
            if output.exists() && receipt.exists() {
                validate_hashes(&directory, &receipt, &output)?;
            } else {
                if output.exists() {
                    fs::remove_dir_all(&output).map_err(error)?;
                }
                match stage {
                    DeclarationIngestionStage::Clean => {
                        DeclarationCleanerService::new(storage.clone())
                            .clean_declarations()
                            .await?;
                    }
                    DeclarationIngestionStage::Resolve => {
                        DeclarationResolverService::new(
                            storage.clone(),
                            WeldrsFunderScorer::new()?,
                            self.candidate_budget,
                        )
                        .resolve_declarations()
                        .await?;
                    }
                    DeclarationIngestionStage::Fetch
                    | DeclarationIngestionStage::Load
                    | DeclarationIngestionStage::Sync
                    | DeclarationIngestionStage::Reconcile => unreachable!("derived stages only"),
                }
                sync_tree(&output)?;
                let hashes = directory_hashes(&directory, &output)?;
                durable_json(&receipt, &hashes)?;
            }
        }
        self.read_snapshot(&directory.join("capture.json"), None)?;
        storage.read_declaration_load().await
    }
}

impl DeclarationSyncRepository for ExposedDatabase {
    async fn sync_head(&self) -> Result<Option<DeclarationSyncHead>, EntityIngestionError> {
        let row = sqlx::query!("SELECT run.revision, run.manifest_path, run.manifest_sha256 FROM exposed.declaration_sync_head head JOIN exposed.declaration_sync_runs run USING (ingestion_key) WHERE head.singleton").fetch_optional(self.pool()).await.map_err(error)?;
        Ok(row.map(|row| DeclarationSyncHead {
            revision: row.revision,
            manifest_path: row.manifest_path.into(),
            manifest_sha256: row.manifest_sha256,
        }))
    }
    async fn publish_sync(
        &self,
        publication: &PreparedPublication,
        load: Option<&DeclarationLoad>,
    ) -> Result<bool, EntityIngestionError> {
        self.publish_declaration_sync(publication, load).await
    }
}

fn directory_hashes(
    root: &Path,
    directory: &Path,
) -> Result<BTreeMap<PathBuf, String>, EntityIngestionError> {
    let mut result = BTreeMap::new();
    for entry in fs::read_dir(directory).map_err(error)? {
        let path = entry.map_err(error)?.path();
        if !path.is_file() {
            return Err(error("unexpected derived output directory"));
        }
        result.insert(
            path.strip_prefix(root).map_err(error)?.into(),
            sha256_digest(&fs::read(path).map_err(error)?),
        );
    }
    Ok(result)
}
fn validate_hashes(root: &Path, receipt: &Path, output: &Path) -> Result<(), EntityIngestionError> {
    let hashes: BTreeMap<PathBuf, String> =
        serde_json::from_slice(&fs::read(receipt).map_err(error)?).map_err(error)?;
    if hashes != directory_hashes(root, output)? {
        return Err(error("derived stage inventory or digest mismatch"));
    }
    Ok(())
}

fn durable_json(path: &Path, value: &impl serde::Serialize) -> Result<(), EntityIngestionError> {
    let temporary = path.with_extension("staging");
    fs::write(&temporary, serde_json::to_vec_pretty(value).map_err(error)?).map_err(error)?;
    fs::File::open(&temporary)
        .and_then(|file| file.sync_all())
        .map_err(error)?;
    fs::rename(temporary, path).map_err(error)?;
    fs::File::open(
        path.parent()
            .ok_or_else(|| error("artifact has no parent"))?,
    )
    .and_then(|file| file.sync_all())
    .map_err(error)
}
fn sync_tree(directory: &Path) -> Result<(), EntityIngestionError> {
    for entry in fs::read_dir(directory).map_err(error)? {
        let path = entry.map_err(error)?.path();
        if path.is_dir() {
            sync_tree(&path)?;
        } else {
            fs::File::open(path)
                .and_then(|file| file.sync_all())
                .map_err(error)?;
        }
    }
    fs::File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(error)
}
fn error(error: impl std::fmt::Display) -> EntityIngestionError {
    EntityIngestionError::DataError(error.to_string())
}
