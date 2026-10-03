use std::fs::{self, File};

use arrow::{
    array::AsArray,
    datatypes::{DataType, TimeUnit},
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
            CapturedDeclaration, CapturedVersion, DeclarationId, DeclarationMemberOutput,
            SourceFieldGroup, StoredMember,
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
    let member = StoredMember::new(Uuid::from_u128(1), 512)?;
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
        &DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into()))
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
    assert_eq!(manifest["row_count"], 0);
    assert_eq!(
        manifest["members"],
        json!([{
            "member_id": member.member_id().to_string(),
            "parliament_member_id": 512,
            "file": format!("{}.parquet", member.member_id()),
            "declaration_count": 0,
            "row_count": 0
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
    let member = StoredMember::new(Uuid::from_u128(1), 512)?;
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

#[tokio::test]
async fn saves_the_same_declaration_for_each_declaring_member() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let members = [
        StoredMember::new(Uuid::from_u128(1), 4613)?,
        StoredMember::new(Uuid::from_u128(2), 5030)?,
    ];
    let declaration = CapturedDeclaration::new(
        DeclarationId::new(42)?,
        None,
        3,
        "Donations".into(),
        vec![CapturedVersion::new(
            0,
            820,
            NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
            None,
            vec![SourceFieldGroup::new(
                "/versions/0/fields".into(),
                None,
                Some("Shared donor".into()),
                None,
                Some("2000.00".into()),
                Some("GBP".into()),
                None,
                None,
                None,
                None,
            )],
        )?],
        Utc::now(),
        r#"{"id":42}"#.into(),
    )?;

    storage.begin_declarations().await?;
    for member in members {
        storage
            .write_raw_declarations(member, std::slice::from_ref(&declaration))
            .await?;
        let file = File::open(storage.declarations_path().join(member_filename(member)))?;
        let mut reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;
        let row = reader.next().unwrap()?;
        let member_ids = row.column_by_name("member_id").unwrap().as_string::<i32>();
        let declaration_ids = row
            .column_by_name("declaration_id")
            .unwrap()
            .as_primitive::<arrow::datatypes::UInt32Type>();

        assert_eq!(row.num_rows(), 1);
        assert_eq!(member_ids.value(0), member.member_id().to_string());
        assert_eq!(declaration_ids.value(0), 42);
        assert!(reader.next().is_none());
    }
    let outputs = members.map(|member| DeclarationMemberOutput::new(member, 1, 1));
    storage.complete_declarations(&outputs).await?;
    let manifest: Value = serde_json::from_slice(&fs::read(
        storage.declarations_path().join("manifest.json"),
    )?)?;
    assert_eq!(manifest["member_count"], 2);
    assert_eq!(manifest["declaration_count"], 2);
    assert_eq!(manifest["row_count"], 2);
    Ok(())
}
