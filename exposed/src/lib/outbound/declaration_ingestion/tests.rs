use std::fs::{self, File};

use arrow::{
    datatypes::{DataType, TimeUnit},
    json::ArrayWriter,
};
use chrono::{NaiveDate, Utc};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::{Value, json};
use tempfile::TempDir;
use uuid::Uuid;

use super::{ExposedDataPipeline, member_filename};
use crate::domain::{
    models::{
        declaration_ingestion::{
            CapturedDeclaration, CapturedFundingEntry, DeclarationId, DeclarationMemberOutput,
            MemberAsId,
        },
        entity_ingestion::IngestionKey,
    },
    repositories::entity_ingestion::EntityIngestionStorage,
};

#[tokio::test]
async fn publishes_empty_member_partitions_and_completion_counts() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let key = IngestionKey::default();
    let storage =
        ExposedDataPipeline::new_with_ingestion_key(&tmp.path().to_path_buf(), key.clone())?;
    let member = MemberAsId::new(Uuid::from_u128(1), 512)?;
    let directory = storage.declarations_path();

    storage.begin_declarations().await?;
    storage.write_raw_declarations(member, &[]).await?;

    assert!(!directory.join("manifest.json").exists());
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

    storage
        .complete_declarations(&[DeclarationMemberOutput::new(member, 0, 0)])
        .await?;

    let manifest: Value = serde_json::from_slice(&fs::read(directory.join("manifest.json"))?)?;
    assert_eq!(manifest["ingestion_key"], key.to_string());
    assert_eq!(manifest["member_count"], 1);
    assert_eq!(manifest["declaration_count"], 0);
    assert_eq!(manifest["funding_entry_count"], 0);
    assert_eq!(
        manifest["members"],
        json!([{
            "member_id": member.member_id().to_string(),
            "parliament_member_id": 512,
            "file": format!("{}.parquet", member.member_id()),
            "declaration_count": 0,
            "funding_entry_count": 0
        }])
    );
    Ok(())
}

#[tokio::test]
async fn protects_existing_partitions_and_completed_or_incomplete_runs() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let member = MemberAsId::new(Uuid::from_u128(1), 512)?;
    let directory = storage.declarations_path();
    let manifest_path = directory.join("manifest.json");
    let partition_path = directory.join(member_filename(member));

    storage.begin_declarations().await?;
    storage.write_raw_declarations(member, &[]).await?;
    let original_partition = fs::read(&partition_path)?;

    assert!(storage.begin_declarations().await.is_err());
    assert!(storage.write_raw_declarations(member, &[]).await.is_err());
    assert_eq!(fs::read(&partition_path)?, original_partition);
    assert!(!manifest_path.exists());

    let outputs = [DeclarationMemberOutput::new(member, 0, 0)];
    storage.complete_declarations(&outputs).await?;
    let original_manifest = fs::read(&manifest_path)?;

    assert!(storage.begin_declarations().await.is_err());
    assert!(storage.complete_declarations(&outputs).await.is_err());
    assert_eq!(fs::read(&manifest_path)?, original_manifest);
    assert_eq!(fs::read(&partition_path)?, original_partition);
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
async fn saves_one_record_per_mp_with_nested_funding_entries() -> anyhow::Result<()> {
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

    storage.begin_declarations().await?;
    for member in members {
        storage
            .write_raw_declarations(member, &declarations)
            .await?;
        let records =
            read_declarations(&storage.declarations_path().join(member_filename(member)))?;
        let [funded, nonfinancial] = records.as_slice() else {
            panic!("expected one record per declaration")
        };
        assert_eq!(funded["member_id"], member.member_id().to_string());
        assert_eq!(funded["declaration_id"], 42);
        assert_eq!(funded["register_id"], 820);
        assert_eq!(funded["register_published_date"], "2026-09-07");
        assert_eq!(funded["registration_date"], "2026-09-01");
        assert_eq!(
            funded["funding_entries"],
            json!([
                {"donor_name": "First donor", "amount": "2000.00", "currency": "GBP"},
                {"donor_name": "Second donor", "amount": "3000.00", "currency": "GBP"}
            ])
        );
        assert_eq!(nonfinancial["member_id"], member.member_id().to_string());
        assert_eq!(nonfinancial["declaration_id"], 43);
        assert_eq!(nonfinancial["funding_entries"], json!([]));
    }
    let outputs = members.map(|member| DeclarationMemberOutput::new(member, 2, 2));
    storage.complete_declarations(&outputs).await?;
    let manifest: Value = serde_json::from_slice(&fs::read(
        storage.declarations_path().join("manifest.json"),
    )?)?;
    assert_eq!(manifest["schema_version"], 2);
    assert_eq!(manifest["member_count"], 2);
    assert_eq!(manifest["declaration_count"], 4);
    assert_eq!(manifest["funding_entry_count"], 4);
    Ok(())
}
