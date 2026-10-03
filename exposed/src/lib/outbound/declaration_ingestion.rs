//! Parquet partitions and completion manifests for raw declaration captures.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use arrow::{
    array::{
        BooleanArray, Date32Array, RecordBatch, StringArray, TimestampMicrosecondArray, UInt32Array,
    },
    datatypes::{DataType, Field, Schema, TimeUnit},
};
use chrono::{Datelike, Utc};
use parquet::arrow::async_writer::AsyncArrowWriter;
use serde::Serialize;
use tokio::{fs, io::AsyncWriteExt};

use super::file_system::ExposedDataPipeline;
use crate::domain::{
    models::declaration_ingestion::{
        CapturedDeclaration, DeclarationId, DeclarationMemberOutput, SourceFieldGroup, StoredMember,
    },
    repositories::entity_ingestion::EntitySearchPipelineError,
};

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
        member: StoredMember,
        declarations: &[CapturedDeclaration],
    ) -> Result<(), EntitySearchPipelineError> {
        let destination = self.declarations_path().join(member_filename(member));
        let temporary = destination.with_extension("parquet.partial");
        let write = async {
            let file = fs::File::create_new(&temporary).await?;
            let schema = Arc::new(declarations_schema());
            let mut writer = AsyncArrowWriter::try_new(file, schema.clone(), None)?;
            for declaration in declarations {
                anyhow::ensure!(
                    declaration.member() == member,
                    "declaration belongs to a different stored member"
                );
                writer
                    .write(&declaration_batch(declaration, schema.clone())?)
                    .await?;
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
                    row_count: output.row_count(),
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
                schema_version: 1,
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
                row_count: members.iter().map(DeclarationMemberOutput::row_count).sum(),
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

fn member_filename(member: StoredMember) -> String {
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
    row_count: usize,
    members: Vec<MemberManifest>,
}

#[derive(Serialize)]
struct MemberManifest {
    member_id: String,
    parliament_member_id: u32,
    file: String,
    declaration_count: usize,
    row_count: usize,
}

fn declarations_schema() -> Schema {
    Schema::new(vec![
        Field::new("member_id", DataType::Utf8, false),
        Field::new("parliament_member_id", DataType::UInt32, false),
        Field::new("declaration_id", DataType::UInt32, false),
        Field::new("parent_declaration_id", DataType::UInt32, true),
        Field::new("category_id", DataType::UInt32, false),
        Field::new("category_name", DataType::Utf8, false),
        Field::new("version_index", DataType::UInt32, false),
        Field::new("register_id", DataType::UInt32, false),
        Field::new("register_published_date", DataType::Date32, false),
        Field::new("registration_date", DataType::Date32, true),
        Field::new("source_field_path", DataType::Utf8, false),
        Field::new("ultimate_payer_name", DataType::Utf8, true),
        Field::new("donor_name", DataType::Utf8, true),
        Field::new("payer_name", DataType::Utf8, true),
        Field::new("amount", DataType::Utf8, true),
        Field::new("currency", DataType::Utf8, true),
        Field::new("payment_type", DataType::Utf8, true),
        Field::new("funder_kind", DataType::Utf8, true),
        Field::new("company_number", DataType::Utf8, true),
        Field::new("is_ultimate_payer_different", DataType::Boolean, true),
        Field::new(
            "fetched_at",
            DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new("source_json", DataType::Utf8, false),
    ])
}

fn declaration_batch(
    declaration: &CapturedDeclaration,
    schema: Arc<Schema>,
) -> anyhow::Result<RecordBatch> {
    let rows = declaration
        .versions()
        .iter()
        .flat_map(|version| version.groups().iter().map(move |group| (version, group)))
        .collect::<Vec<_>>();
    let text = |field: fn(&SourceFieldGroup) -> Option<&str>| {
        Arc::new(StringArray::from(
            rows.iter()
                .map(|(_, group)| field(group))
                .collect::<Vec<_>>(),
        ))
    };
    let count = rows.len();
    let member_id = declaration.member().member_id().to_string();
    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec![member_id.as_str(); count])),
            Arc::new(UInt32Array::from(vec![
                declaration
                    .member()
                    .parliament_member_id();
                count
            ])),
            Arc::new(UInt32Array::from(vec![declaration.id().value(); count])),
            Arc::new(UInt32Array::from(vec![
                declaration
                    .parent_id()
                    .map(DeclarationId::value);
                count
            ])),
            Arc::new(UInt32Array::from(vec![declaration.category_id(); count])),
            Arc::new(StringArray::from(vec![declaration.category_name(); count])),
            Arc::new(UInt32Array::from(
                rows.iter()
                    .map(|(version, _)| version.index())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(UInt32Array::from(
                rows.iter()
                    .map(|(version, _)| version.register_id())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Date32Array::from(
                rows.iter()
                    .map(|(version, _)| version.published_date().num_days_from_ce() - 719_163)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Date32Array::from(
                rows.iter()
                    .map(|(version, _)| {
                        version
                            .registration_date()
                            .map(|date| date.num_days_from_ce() - 719_163)
                    })
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|(_, group)| group.path())
                    .collect::<Vec<_>>(),
            )),
            text(SourceFieldGroup::ultimate_payer_name),
            text(SourceFieldGroup::donor_name),
            text(SourceFieldGroup::payer_name),
            text(SourceFieldGroup::amount),
            text(SourceFieldGroup::currency),
            text(SourceFieldGroup::payment_type),
            text(SourceFieldGroup::funder_kind),
            text(SourceFieldGroup::company_number),
            Arc::new(BooleanArray::from(
                rows.iter()
                    .map(|(_, group)| group.is_ultimate_payer_different())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(
                TimestampMicrosecondArray::from(vec![
                    declaration.fetched_at().timestamp_micros();
                    count
                ])
                .with_timezone("UTC"),
            ),
            Arc::new(StringArray::from(vec![declaration.source_json(); count])),
        ],
    )?)
}
