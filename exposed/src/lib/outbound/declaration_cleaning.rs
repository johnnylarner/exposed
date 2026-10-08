//! Parquet replay and publication for offline declaration cleaning.

use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    path::Path,
    sync::Arc,
};

use arrow::{
    datatypes::{DataType, Field, Schema},
    json::{ArrayWriter, ReaderBuilder},
};
use chrono::{DateTime, NaiveDate, Utc};
use parquet::arrow::{
    arrow_reader::ParquetRecordBatchReaderBuilder, async_writer::AsyncArrowWriter,
};
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use super::{
    ExposedDataPipeline, declaration_ingestion::declarations_schema,
    declaration_source::replay_declaration,
};
use crate::domain::{
    models::{
        declaration_cleaning::{CapturedMemberDeclarations, CleanedDeclarations},
        declaration_ingestion::{
            CapturedDeclaration, CapturedFundingEntry, DeclarationId, MemberAsId,
        },
    },
    repositories::{
        declaration_cleaning::DeclarationCleaningStorage,
        entity_ingestion::EntitySearchPipelineError,
    },
};

impl DeclarationCleaningStorage for ExposedDataPipeline {
    async fn read_captured_declarations(
        &self,
    ) -> Result<Vec<CapturedMemberDeclarations>, EntitySearchPipelineError> {
        if fs::try_exists(self.cleaned_declarations_path())
            .await
            .map_err(read_error)?
        {
            return Err(write_error("cleaned declarations already exist"));
        }
        let directory = self.raw_path().join("declarations");
        let mut files = fs::read_dir(&directory).await.map_err(read_error)?;
        let mut paths = Vec::new();
        while let Some(file) = files.next_entry().await.map_err(read_error)? {
            let path = file.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "parquet")
            {
                paths.push(path);
            }
        }
        paths.sort();
        if paths.is_empty() {
            return Err(read_error("no raw declaration partitions found"));
        }
        paths
            .into_iter()
            .map(|path| {
                read_partition(&path)
                    .map_err(|error| read_error(format!("{}: {error}", path.display())))
            })
            .collect()
    }

    async fn publish_cleaned_declarations(
        &self,
        cleaned: &CleanedDeclarations,
    ) -> Result<(), EntitySearchPipelineError> {
        let destination = self.cleaned_declarations_path();
        if fs::try_exists(&destination).await.map_err(write_error)? {
            return Err(write_error("cleaned declarations already exist"));
        }
        let parent = destination
            .parent()
            .expect("cleaned declarations have a parent");
        fs::create_dir_all(parent).await.map_err(write_error)?;
        let staging = parent.join(format!(".declarations-{}", Uuid::now_v7()));
        fs::create_dir(&staging).await.map_err(write_error)?;
        let result = async {
            write_table(
                &staging.join("funding_entries.parquet"),
                funding_schema(),
                &cleaned.funding_entries,
            )
            .await?;
            write_table(
                &staging.join("funders.parquet"),
                funders_schema(),
                &cleaned.funders,
            )
            .await?;
            fs::rename(&staging, &destination)
                .await
                .map_err(write_error)
        }
        .await;
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging).await;
        }
        result
    }
}

#[derive(Deserialize)]
struct RawDeclarationRecord {
    member_id: Uuid,
    parliament_member_id: u32,
    declaration_id: u32,
    parent_declaration_id: Option<u32>,
    category_id: u32,
    category_name: String,
    register_id: u32,
    register_published_date: NaiveDate,
    registration_date: Option<NaiveDate>,
    fetched_at: DateTime<Utc>,
    source_json: String,
    #[serde(flatten)]
    funding: CapturedFundingEntry,
}

impl RawDeclarationRecord {
    fn matches(&self, declaration: &CapturedDeclaration) -> bool {
        self.declaration_id == declaration.id().value()
            && self.parent_declaration_id == declaration.parent_id().map(DeclarationId::value)
            && self.category_id == declaration.category_id()
            && self.category_name == declaration.category_name()
            && self.register_id == declaration.register_id()
            && self.register_published_date == declaration.register_published_date()
            && self.registration_date == declaration.registration_date()
            && self.fetched_at == declaration.fetched_at()
    }
}

pub(super) fn read_partition(
    path: &Path,
) -> Result<CapturedMemberDeclarations, EntitySearchPipelineError> {
    let member_id = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| read_error("partition filename must contain a member UUID"))?
        .parse::<Uuid>()
        .map_err(read_error)?;
    if member_id.is_nil() {
        return Err(read_error("partition member UUID must not be nil"));
    }
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path).map_err(read_error)?)
        .map_err(read_error)?;
    if declarations_schema().fields().iter().any(|required| {
        reader
            .schema()
            .field_with_name(required.name())
            .map_or(true, |actual| {
                actual.data_type() != required.data_type()
                    || actual.is_nullable() != required.is_nullable()
            })
    }) {
        return Err(read_error(
            "raw declaration schema does not match the captured format",
        ));
    }
    let mut groups = BTreeMap::<u32, Vec<RawDeclarationRecord>>::new();
    let mut member = None;
    for batch in reader.build().map_err(read_error)? {
        let mut json = ArrayWriter::new(Vec::new());
        json.write(&batch.map_err(read_error)?)
            .map_err(read_error)?;
        json.finish().map_err(read_error)?;
        let rows: Vec<RawDeclarationRecord> =
            serde_json::from_slice(&json.into_inner()).map_err(read_error)?;
        for row in rows {
            let identity =
                MemberAsId::new(row.member_id, row.parliament_member_id).map_err(read_error)?;
            if row.member_id != member_id || member.is_some_and(|member| member != identity) {
                return Err(read_error(
                    "partition member identity disagrees with its rows",
                ));
            }
            member = Some(identity);
            groups.entry(row.declaration_id).or_default().push(row);
        }
    }
    let Some(member) = member else {
        return Ok(CapturedMemberDeclarations::Empty { member_id });
    };
    let mut declarations = Vec::with_capacity(groups.len());
    for (id, rows) in groups {
        let first = &rows[0];
        let source = serde_json::from_str(&first.source_json).map_err(read_error)?;
        let evidence = replay_declaration(source, first.fetched_at).map_err(read_error)?;
        let captured = evidence.declaration();
        if rows
            .iter()
            .any(|row| row.source_json != first.source_json || !row.matches(captured))
        {
            return Err(read_error(format!(
                "declaration {id}: source metadata disagrees with raw projection"
            )));
        }
        let actual = funding_counts(rows.iter().map(|row| &row.funding));
        let placeholder = CapturedFundingEntry::default();
        let expected = if captured.funding_entries().is_empty() {
            funding_counts([&placeholder])
        } else {
            funding_counts(captured.funding_entries())
        };
        if actual != expected {
            return Err(read_error(format!(
                "declaration {id}: funding multiplicity or values disagree with source"
            )));
        }
        declarations.push(evidence);
    }
    Ok(CapturedMemberDeclarations::Populated {
        member,
        declarations,
    })
}

fn funding_counts<'a>(
    entries: impl IntoIterator<Item = &'a CapturedFundingEntry>,
) -> HashMap<&'a CapturedFundingEntry, usize> {
    let mut counts = HashMap::new();
    for entry in entries {
        *counts.entry(entry).or_insert(0) += 1;
    }
    counts
}

pub(super) async fn write_table<T: Serialize + Sync>(
    path: &Path,
    schema: Schema,
    rows: &[T],
) -> Result<(), EntitySearchPipelineError> {
    let file = fs::File::create_new(path).await.map_err(write_error)?;
    let schema = Arc::new(schema);
    let mut writer = AsyncArrowWriter::try_new(file, schema.clone(), None).map_err(write_error)?;
    let mut decoder = ReaderBuilder::new(schema)
        .build_decoder()
        .map_err(write_error)?;
    for chunk in rows.chunks(1024) {
        decoder.serialize(chunk).map_err(write_error)?;
        if let Some(batch) = decoder.flush().map_err(write_error)? {
            writer.write(&batch).await.map_err(write_error)?;
        }
    }
    writer.close().await.map_err(write_error)?;
    Ok(())
}

pub(super) fn funding_schema() -> Schema {
    let mut fields = vec![
        Field::new("funding_entry_id", DataType::Utf8, false),
        Field::new("funding_ordinal", DataType::UInt32, false),
        Field::new("donor_funder_id", DataType::Utf8, true),
        Field::new("payer_funder_id", DataType::Utf8, true),
        Field::new("ultimate_payer_funder_id", DataType::Utf8, true),
    ];
    fields.extend(
        declarations_schema()
            .fields()
            .iter()
            .filter(|field| field.name() != "source_json")
            .map(|field| field.as_ref().clone()),
    );
    Schema::new(fields)
}

pub(super) fn funders_schema() -> Schema {
    let text = |name, nullable| Field::new(name, DataType::Utf8, nullable);
    let list = |name| Field::new(name, DataType::List(Arc::new(text("item", false))), false);
    Schema::new(vec![
        text("funder_id", false),
        text("member_id", false),
        Field::new("declaration_id", DataType::UInt32, false),
        Field::new("register_id", DataType::UInt32, false),
        text("role", false),
        text("source_scope", false),
        text("source_pointer", false),
        text("funding_entry_id", true),
        Field::new("funding_ordinal", DataType::UInt32, true),
        text("donor_kind", true),
        text("donor_company_number", true),
        text("address_match_quality", false),
        text("address_raw", true),
        text("address_normalized", true),
        text("address_source_field", true),
        text("name_raw", true),
        text("name_status", false),
        text("name_normalized", true),
        text("name_accent_folded", true),
        text("name_tokens", true),
        text("name_token_key", true),
        text("primary_name_normalized", true),
        text("organisation_core", true),
        text("person_core", true),
        text("person_initials", true),
        text("organisation_initials", true),
        text("explicit_acronym", true),
        text("acronym_expansion_normalized", true),
        list("parenthetical_text"),
        list("explicit_aliases"),
        list("alias_normalized"),
        Field::new("has_conjunction", DataType::Boolean, false),
        Field::new("has_unbalanced_parentheses", DataType::Boolean, false),
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
