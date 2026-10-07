//! Parquet publication and the versioned Python Splink scoring protocol.

use super::{
    ExposedDataPipeline,
    declaration_cleaning::{funders_schema, funding_schema, write_table},
};
use crate::domain::{
    models::{
        declaration_resolution::{
            Observation, Payment, ResolutionInput, ResolvedDeclarations, ScoredPair, ScoredPairs,
            ScoringInput,
        },
        entity_ingestion::EntityIngestionError,
    },
    repositories::{
        declaration_resolution::{DeclarationResolutionStorage, FunderScorer},
        entity_ingestion::EntitySearchPipelineError,
    },
};
use arrow::{
    datatypes::{DataType, Field, Schema},
    json::ArrayWriter,
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::{Deserialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tokio::fs;
use uuid::Uuid;

impl DeclarationResolutionStorage for ExposedDataPipeline {
    async fn read_resolution_input(&self) -> Result<ResolutionInput, EntityIngestionError> {
        let destination = self.resolved_declarations_path();
        if fs::try_exists(&destination).await.map_err(read_error)? {
            return Err(write_error("resolved declarations already exist").into());
        }
        let directory = self.cleaned_declarations_path();
        let (observations, observation_digest) =
            read_table::<Observation>(&directory.join("funders.parquet"), &funders_schema())?;
        let (payments, payment_digest) = read_table::<Payment>(
            &directory.join("funding_entries.parquet"),
            &funding_schema(),
        )?;
        ResolutionInput::new(
            observations,
            payments,
            BTreeMap::from([
                ("funders.parquet".into(), observation_digest),
                ("funding_entries.parquet".into(), payment_digest),
            ]),
        )
    }
    fn resolution_run_key(&self) -> String {
        crate::domain::models::entity_ingestion::IngestionKey::from(self.key()).to_string()
    }
    async fn publish_resolution(
        &self,
        result: &ResolvedDeclarations,
    ) -> Result<(), EntitySearchPipelineError> {
        let destination = self.resolved_declarations_path();
        let parent = destination
            .parent()
            .expect("resolved declarations have a parent");
        fs::create_dir_all(parent).await.map_err(write_error)?;
        let staging = parent.join(format!(".declarations-{}", Uuid::now_v7()));
        let publication = async {
            if fs::try_exists(&destination).await.map_err(write_error)? {
                return Err(write_error("resolved declarations already exist"));
            }
            fs::create_dir(&staging).await.map_err(write_error)?;
            write_table(
                &staging.join("observation_resolution.parquet"),
                observation_schema(),
                &result.observations,
            )
            .await?;
            write_table(
                &staging.join("payment_attribution.parquet"),
                attribution_schema(),
                &result.attributions,
            )
            .await?;
            write_table(
                &staging.join("pair_decisions.parquet"),
                pair_schema(),
                &result.pairs,
            )
            .await?;
            fs::write(
                staging.join("manifest.json"),
                serde_json::to_vec_pretty(&result.manifest).map_err(write_error)?,
            )
            .await
            .map_err(write_error)?;
            publish_directory(&staging, &destination).map_err(write_error)
        }
        .await;
        if publication.is_err() {
            let _ = fs::remove_dir_all(&staging).await;
        }
        publication
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn publish_directory(staging: &Path, destination: &Path) -> std::io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let staging = CString::new(staging.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    #[cfg(target_os = "macos")]
    let result =
        unsafe { libc::renamex_np(staging.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            staging.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn publish_directory(_staging: &Path, _destination: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "atomic resolution publication requires macOS or Linux",
    ))
}

fn read_table<T: DeserializeOwned>(
    path: &Path,
    expected: &Schema,
) -> Result<(Vec<T>, String), EntitySearchPipelineError> {
    let bytes = std::fs::read(path).map_err(read_error)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let reader =
        ParquetRecordBatchReaderBuilder::try_new(bytes::Bytes::from(bytes)).map_err(read_error)?;
    for required in expected.fields() {
        let actual=reader.schema().field_with_name(required.name()).map_err(|_|read_error("cleaned declarations lack the resolution schema; clean the retained raw capture into a fresh ingestion run"))?;
        if actual.data_type() != required.data_type()
            || actual.is_nullable() != required.is_nullable()
        {
            return Err(read_error(format!(
                "cleaned field {} has an incompatible type; clean retained raw into a fresh run",
                required.name()
            )));
        }
    }
    let mut rows = Vec::new();
    for batch in reader.build().map_err(read_error)? {
        let mut writer = ArrayWriter::new(Vec::new());
        writer
            .write(&batch.map_err(read_error)?)
            .map_err(read_error)?;
        writer.finish().map_err(read_error)?;
        rows.extend(serde_json::from_slice::<Vec<T>>(&writer.into_inner()).map_err(read_error)?);
    }
    Ok((rows, digest))
}

/// Python subprocess adapter for the pinned Splink worker.
pub struct SplinkScorer {
    python: PathBuf,
    worker: PathBuf,
}
impl SplinkScorer {
    /// Selects the interpreter and the separately installed worker script.
    ///
    /// # Errors
    /// Returns an error when the configured worker cannot be read.
    pub fn new(python: PathBuf, worker: PathBuf) -> Result<Self, EntityIngestionError> {
        if !worker.is_file() {
            return Err(EntityIngestionError::DataError(format!(
                "Splink worker missing at {}; configure resolution_worker",
                worker.display()
            )));
        }
        Ok(Self { python, worker })
    }
}
#[derive(Deserialize)]
struct WorkerOutput {
    version: u32,
    model: serde_json::Value,
    pairs: Vec<ScoredPair>,
}
impl FunderScorer for SplinkScorer {
    async fn score(&self, input: &ScoringInput) -> Result<ScoredPairs, EntityIngestionError> {
        let encoded = serde_json::to_vec(input)
            .map_err(|e| EntityIngestionError::DataError(e.to_string()))?;
        let python = self.python.clone();
        let worker = self.worker.clone();
        let output=tokio::task::spawn_blocking(move || -> Result<WorkerOutput,EntityIngestionError> {
            let temporary=tempfile::tempdir().map_err(|e|EntityIngestionError::DataError(e.to_string()))?;
            let request=temporary.path().join("input.json"); let response=temporary.path().join("output.json");
            std::fs::write(&request,encoded).map_err(|e|EntityIngestionError::DataError(e.to_string()))?;
            let process=Command::new(&python).arg(&worker).arg(&request).arg(&response).env("PYTHONDONTWRITEBYTECODE","1").output().map_err(|e|EntityIngestionError::DataError(format!("cannot run Splink interpreter {}: {e}; install resolution dependencies before running resolve",python.display())))?;
            if !process.status.success() { return Err(EntityIngestionError::DataError(format!("Splink worker failed ({}): {}",process.status,String::from_utf8_lossy(&process.stderr)))); }
            let bytes=std::fs::read(&response).map_err(|e|EntityIngestionError::DataError(format!("Splink worker output missing: {e}")))?;
            serde_json::from_slice(&bytes).map_err(|e|EntityIngestionError::DataError(format!("invalid Splink worker output: {e}")))
        }).await.map_err(|e|EntityIngestionError::DataError(e.to_string()))??;
        let expected: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../resolution/exposed_resolution/model.json"
        )))
        .map_err(|error| {
            EntityIngestionError::DataError(format!("invalid bundled frozen model: {error}"))
        })?;
        if output.version != 1
            || [
                "model_version",
                "splink_version",
                "duckdb_version",
                "calibrated",
                "settings",
            ]
            .iter()
            .any(|key| output.model[*key] != expected[*key])
            || output.model["python_version"]
                .as_str()
                .is_none_or(str::is_empty)
        {
            return Err(EntityIngestionError::DataError(
                "unsupported Splink protocol, frozen model, or runtime version".into(),
            ));
        }

        ScoredPairs::checked(output.pairs, output.model, input)
    }
}
fn text(name: &str, nullable: bool) -> Field {
    Field::new(name, DataType::Utf8, nullable)
}
fn observation_schema() -> Schema {
    Schema::new(vec![
        text("funder_id", false),
        text("identity_id", true),
        text("identity_basis", false),
    ])
}
fn attribution_schema() -> Schema {
    Schema::new(vec![
        text("funding_entry_id", false),
        text("attribution_status", false),
        text("selected_funder_id", true),
        text("attribution_basis", true),
        Field::new("selected_parent_declaration_id", DataType::UInt32, true),
        text("unavailable_reason", true),
        Field::new(
            "issues",
            DataType::List(Arc::new(text("item", false))),
            false,
        ),
    ])
}
fn pair_schema() -> Schema {
    Schema::new(vec![
        text("left_funder_id", false),
        text("right_funder_id", false),
        Field::new("probability", DataType::Float64, false),
        Field::new("name_level", DataType::Int32, false),
        Field::new("address_level", DataType::Int32, false),
        text("disposition", false),
        text("reason", false),
    ])
}
fn read_error(error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::ReadError(error.to_string())
}
fn write_error(error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::WriteError(error.to_string())
}

#[cfg(test)]
mod tests;
