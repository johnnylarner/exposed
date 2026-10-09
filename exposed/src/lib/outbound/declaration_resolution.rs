use super::{
    ExposedDataPipeline,
    declaration_cleaning::{funders_schema, funding_schema, write_table},
    file_system::publish_directory,
};
use crate::domain::{
    models::{
        declaration_resolution::{Observation, Payment, ResolutionInput, ResolvedDeclarations},
        entity_ingestion::EntityIngestionError,
    },
    repositories::{
        declaration_resolution::DeclarationResolutionStorage,
        entity_ingestion::EntitySearchPipelineError,
    },
};
use arrow::{
    datatypes::{DataType, Field, Schema},
    json::ArrayWriter,
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::Arc};
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

pub(super) fn read_table<T: DeserializeOwned>(
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

fn text(name: &str, nullable: bool) -> Field {
    Field::new(name, DataType::Utf8, nullable)
}
pub(super) fn observation_schema() -> Schema {
    Schema::new(vec![
        text("funder_id", false),
        text("identity_id", true),
        text("identity_basis", false),
    ])
}
pub(super) fn attribution_schema() -> Schema {
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
pub(super) fn pair_schema() -> Schema {
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
