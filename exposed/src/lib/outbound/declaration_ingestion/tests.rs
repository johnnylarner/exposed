use std::fs::File;

use arrow::{
    datatypes::{DataType, TimeUnit},
    json::ArrayWriter,
};
use chrono::{NaiveDate, Utc};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::Value;
use tempfile::TempDir;
use uuid::Uuid;

use super::{ExposedDataPipeline, member_filename};
use crate::domain::{
    models::{
        declaration_ingestion::{
            CapturedDeclaration, CapturedFundingEntry, DeclarationId, MemberAsId,
        },
        entity_ingestion::IngestionKey,
    },
    repositories::entity_ingestion::EntityIngestionStorage,
};

#[tokio::test]
async fn writes_a_readable_empty_member_file() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let member = MemberAsId::new(Uuid::from_u128(1), 512)?;
    let directory = storage.declarations_path();

    storage.write_raw_declarations(member, &[]).await?;

    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(
        directory.join(member_filename(member)),
    )?)?;
    let schema = reader.schema();
    assert_eq!(
        schema.field_with_name("registration_date")?.data_type(),
        &DataType::Date32
    );
    assert_eq!(
        schema.field_with_name("fetched_at")?.data_type(),
        &DataType::Timestamp(TimeUnit::Microsecond, Some("+00:00".into()))
    );
    assert_eq!(
        schema.field_with_name("source_json")?.data_type(),
        &DataType::Utf8
    );
    assert_eq!(reader.build()?.count(), 0);

    Ok(())
}

fn read_declarations(path: &std::path::Path) -> anyhow::Result<Vec<Value>> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?.build()?;
    let mut json = ArrayWriter::new(Vec::new());
    for batch in reader {
        json.write(&batch?)?;
    }
    json.finish()?;
    Ok(serde_json::from_slice(&json.into_inner())?)
}

#[tokio::test]
async fn flattens_funding_entries_into_separate_rows() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let members = [
        MemberAsId::new(Uuid::from_u128(1), 4613)?,
        MemberAsId::new(Uuid::from_u128(2), 5030)?,
    ];
    let funding =
        [("First donor", "2000.00"), ("Second donor", "3000.00")].map(|(name, amount)| {
            CapturedFundingEntry::new(
                None,
                Some(name.into()),
                None,
                Some(amount.into()),
                Some("GBP".into()),
                None,
                None,
                None,
                None,
            )
        });
    let declaration = CapturedDeclaration::new(
        DeclarationId::new(42)?,
        None,
        3,
        "Donations".into(),
        820,
        NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
        NaiveDate::from_ymd_opt(2026, 9, 1),
        funding.to_vec(),
        Utc::now(),
        r#"{"id":42}"#.into(),
    )?;
    let nonfinancial = CapturedDeclaration::new(
        DeclarationId::new(43)?,
        None,
        12,
        "Miscellaneous".into(),
        820,
        NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
        None,
        vec![],
        Utc::now(),
        r#"{"id":43}"#.into(),
    )?;
    let declarations = [declaration, nonfinancial];

    for member in members {
        storage
            .write_raw_declarations(member, &declarations)
            .await?;
        let records =
            read_declarations(&storage.declarations_path().join(member_filename(member)))?;
        let [first_funding, second_funding, nonfinancial] = records.as_slice() else {
            panic!("expected one record per declaration")
        };
        assert_eq!(first_funding["member_id"], member.member_id().to_string());
        assert_eq!(first_funding["member_id"], second_funding["member_id"]);

        assert_eq!(first_funding["declaration_id"], 42);
        assert_eq!(
            first_funding["declaration_id"],
            second_funding["declaration_id"],
        );

        assert_eq!(first_funding["register_id"], 820);
        assert_eq!(first_funding["register_id"], second_funding["register_id"],);

        assert_eq!(first_funding["register_published_date"], "2026-09-07");
        assert_eq!(
            first_funding["register_published_date"],
            second_funding["register_published_date"],
        );

        assert_eq!(first_funding["registration_date"], "2026-09-01");
        assert_eq!(
            first_funding["registration_date"],
            second_funding["registration_date"],
        );

        assert_eq!(first_funding["donor_name"], "First donor");
        assert_eq!(second_funding["donor_name"], "Second donor");

        assert_eq!(first_funding["amount"], "2000.00");
        assert_eq!(second_funding["amount"], "3000.00");

        assert_eq!(first_funding["currency"], "GBP");
        assert_eq!(second_funding["currency"], "GBP");

        assert_eq!(nonfinancial["member_id"], member.member_id().to_string());
        assert_eq!(nonfinancial["declaration_id"], 43);
        assert_eq!(nonfinancial["amount"], serde_json::Value::Null);
    }
    Ok(())
}
