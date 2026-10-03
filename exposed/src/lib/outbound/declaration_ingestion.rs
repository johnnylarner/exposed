//! Parquet storage for captured declarations.

use std::{path::PathBuf, sync::Arc};

use arrow::{
    datatypes::{DataType, Field, Schema, TimeUnit},
    json::ReaderBuilder,
};
use chrono::{DateTime, NaiveDate, Utc};
use parquet::arrow::async_writer::AsyncArrowWriter;
use serde::Serialize;
use tokio::fs;

use super::file_system::ExposedDataPipeline;
use crate::domain::{
    models::declaration_ingestion::{
        CapturedDeclaration, CapturedFundingEntry, DeclarationId, MemberAsId,
    },
    repositories::entity_ingestion::EntitySearchPipelineError,
};

#[cfg(test)]
mod tests;

impl ExposedDataPipeline {
    fn declarations_path(&self) -> PathBuf {
        self.raw_path().join("declarations")
    }

    pub(super) async fn write_declaration_partition(
        &self,
        member: MemberAsId,
        declarations: &[CapturedDeclaration],
    ) -> Result<(), EntitySearchPipelineError> {
        let directory = self.declarations_path();
        fs::create_dir_all(&directory).await.map_err(write_error)?;
        let file = fs::File::create_new(directory.join(member_filename(member)))
            .await
            .map_err(write_error)?;
        let schema = Arc::new(declarations_schema());
        let mut writer =
            AsyncArrowWriter::try_new(file, schema.clone(), None).map_err(write_error)?;
        let records = declarations
            .iter()
            .map(|declaration| declaration_record(member, declaration))
            .collect::<Vec<_>>();
        let mut decoder = ReaderBuilder::new(schema)
            .build_decoder()
            .map_err(write_error)?;
        decoder.serialize(&records).map_err(write_error)?;
        if let Some(batch) = decoder.flush().map_err(write_error)? {
            writer.write(&batch).await.map_err(write_error)?;
        }
        writer.close().await.map_err(write_error)?;
        Ok(())
    }
}

fn member_filename(member: MemberAsId) -> String {
    format!("{}.parquet", member.member_id())
}

fn write_error(error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::WriteError(error.to_string())
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
