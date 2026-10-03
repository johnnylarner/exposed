//! Parquet partitions and completion manifests for raw declaration captures.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use arrow::{
    datatypes::{DataType, Field, Schema, TimeUnit},
    json::ReaderBuilder,
};
use chrono::{DateTime, NaiveDate, Utc};
use parquet::arrow::async_writer::AsyncArrowWriter;
use serde::Serialize;
use tokio::{fs, io::AsyncWriteExt};

use super::file_system::ExposedDataPipeline;
use crate::domain::{
    models::declaration_ingestion::{
        CapturedDeclaration, CapturedFundingEntry, DeclarationId, DeclarationMemberOutput,
        MemberAsId,
    },
    repositories::entity_ingestion::EntitySearchPipelineError,
};

#[cfg(test)]
mod tests;

impl ExposedDataPipeline {
    fn declarations_path(&self) -> PathBuf {
        self.raw_path().join("declarations")
    }

    pub(super) async fn begin_declaration_capture(&self) -> Result<(), EntitySearchPipelineError> {
        let path = self.declarations_path();
        fs::create_dir(&path).await.map_err(|error| {
            write_error(&path, format!("cannot reserve declarations output (use a new ingestion key if it already exists): {error}"))
        })
    }

    pub(super) async fn write_declaration_partition(
        &self,
        member: MemberAsId,
        declarations: &[CapturedDeclaration],
    ) -> Result<(), EntitySearchPipelineError> {
        let destination = self.declarations_path().join(member_filename(member));
        let temporary = destination.with_extension("parquet.partial");
        let write = async {
            let file = fs::File::create_new(&temporary).await?;
            let schema = Arc::new(declarations_schema());
            let mut writer = AsyncArrowWriter::try_new(file, schema.clone(), None)?;
            let records = declarations
                .iter()
                .map(|declaration| declaration_record(member, declaration))
                .collect::<Vec<_>>();
            let mut decoder = ReaderBuilder::new(schema).build_decoder()?;
            decoder.serialize(&records)?;
            if let Some(batch) = decoder.flush()? {
                writer.write(&batch).await?;
            }
            // Closing also produces a schema-bearing Parquet file for empty members.
            writer.close().await?;
            publish(&temporary, &destination).await?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        write.map_err(|error| write_error(&destination, error))
    }

    pub(super) async fn finish_declaration_capture(
        &self,
        members: &[DeclarationMemberOutput],
    ) -> Result<String, EntitySearchPipelineError> {
        let directory = self.declarations_path();
        let destination = directory.join("manifest.json");
        let temporary = destination.with_extension("json.partial");
        let write = async {
            anyhow::ensure!(
                !members.is_empty(),
                "cannot complete an empty member cohort"
            );
            let outputs = members
                .iter()
                .map(|output| MemberManifest {
                    member_id: output.member().member_id().to_string(),
                    parliament_member_id: output.member().parliament_member_id(),
                    file: member_filename(output.member()),
                    declaration_count: output.declaration_count(),
                    funding_entry_count: output.funding_entry_count(),
                })
                .collect::<Vec<_>>();
            for output in &outputs {
                anyhow::ensure!(
                    fs::metadata(directory.join(&output.file)).await?.is_file(),
                    "missing member output {}",
                    output.file
                );
            }
            let manifest = CompletionManifest {
                schema_version: 2,
                ingestion_key: crate::domain::models::entity_ingestion::IngestionKey::from(
                    self.key(),
                )
                .to_string(),
                completed_at: Utc::now().to_rfc3339(),
                member_count: members.len(),
                declaration_count: members
                    .iter()
                    .map(DeclarationMemberOutput::declaration_count)
                    .sum(),
                funding_entry_count: members
                    .iter()
                    .map(DeclarationMemberOutput::funding_entry_count)
                    .sum(),
                members: outputs,
            };
            let mut file = fs::File::create_new(&temporary).await?;
            file.write_all(&serde_json::to_vec_pretty(&manifest)?)
                .await?;
            file.sync_all().await?;
            drop(file);
            publish(&temporary, &destination).await?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        write.map_err(|error| write_error(&destination, error))?;
        Ok(directory.display().to_string())
    }
}

// A hard link publishes a fully closed file atomically and fails if the destination
// already exists. Both paths are in the same directory/filesystem.
async fn publish(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    fs::hard_link(temporary, destination).await?;
    // Publication is committed. A cleanup failure must not report an incomplete run
    // after the completion manifest is already visible.
    let _ = fs::remove_file(temporary).await;
    Ok(())
}

fn member_filename(member: MemberAsId) -> String {
    format!("{}.parquet", member.member_id())
}

fn write_error(path: &Path, error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::WriteError(format!("{}: {error}", path.display()))
}

#[derive(Serialize)]
struct CompletionManifest {
    schema_version: u32,
    ingestion_key: String,
    completed_at: String,
    member_count: usize,
    declaration_count: usize,
    funding_entry_count: usize,
    members: Vec<MemberManifest>,
}

#[derive(Serialize)]
struct MemberManifest {
    member_id: String,
    parliament_member_id: u32,
    file: String,
    declaration_count: usize,
    funding_entry_count: usize,
}

fn declarations_schema() -> Schema {
    let funding_entry = DataType::Struct(
        vec![
            Field::new("ultimate_payer_name", DataType::Utf8, true),
            Field::new("donor_name", DataType::Utf8, true),
            Field::new("payer_name", DataType::Utf8, true),
            Field::new("amount", DataType::Utf8, true),
            Field::new("currency", DataType::Utf8, true),
            Field::new("payment_type", DataType::Utf8, true),
            Field::new("funder_kind", DataType::Utf8, true),
            Field::new("company_number", DataType::Utf8, true),
            Field::new("is_ultimate_payer_different", DataType::Boolean, true),
        ]
        .into(),
    );
    Schema::new(vec![
        Field::new("member_id", DataType::Utf8, false),
        Field::new("parliament_member_id", DataType::UInt32, false),
        Field::new("declaration_id", DataType::UInt32, false),
        Field::new("parent_declaration_id", DataType::UInt32, true),
        Field::new("category_id", DataType::UInt32, false),
        Field::new("category_name", DataType::Utf8, false),
        Field::new("register_id", DataType::UInt32, false),
        Field::new("register_published_date", DataType::Date32, false),
        Field::new("registration_date", DataType::Date32, true),
        Field::new_list(
            "funding_entries",
            Field::new("item", funding_entry, false),
            false,
        ),
        Field::new(
            "fetched_at",
            DataType::Timestamp(TimeUnit::Microsecond, Some("+00:00".into())),
            false,
        ),
        Field::new("source_json", DataType::Utf8, false),
    ])
}

#[derive(Serialize)]
struct DeclarationRecord<'a> {
    member_id: String,
    parliament_member_id: u32,
    declaration_id: u32,
    parent_declaration_id: Option<u32>,
    category_id: u32,
    category_name: &'a str,
    register_id: u32,
    register_published_date: NaiveDate,
    registration_date: Option<NaiveDate>,
    funding_entries: &'a [CapturedFundingEntry],
    fetched_at: DateTime<Utc>,
    source_json: &'a str,
}

fn declaration_record(
    member: MemberAsId,
    declaration: &CapturedDeclaration,
) -> DeclarationRecord<'_> {
    DeclarationRecord {
        member_id: member.member_id().to_string(),
        parliament_member_id: member.parliament_member_id(),
        declaration_id: declaration.id().value(),
        parent_declaration_id: declaration.parent_id().map(DeclarationId::value),
        category_id: declaration.category_id(),
        category_name: declaration.category_name(),
        register_id: declaration.register_id(),
        register_published_date: declaration.register_published_date(),
        registration_date: declaration.registration_date(),
        funding_entries: declaration.funding_entries(),
        fetched_at: declaration.fetched_at(),
        source_json: declaration.source_json(),
    }
}
