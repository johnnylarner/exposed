use serde_json::json;
use tempfile::TempDir;

use super::{ExposedDataPipeline, funding_schema, replay_declaration, write_table};
use crate::domain::models::{
    declaration_cleaning::{
        CapturedMemberDeclarations, CleanedDeclarations, FunderObservationSource,
    },
    entity_ingestion::IngestionKey,
    parliament_member::MemberId,
};

#[tokio::test]
async fn opening_missing_input_does_not_create_an_ingestion_run() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let root = tmp.path().join("missing");
    assert!(
        ExposedDataPipeline::open_existing_declarations(&root, IngestionKey::default()).is_err()
    );
    assert!(!root.exists());
    Ok(())
}

#[tokio::test]
async fn closes_an_empty_table_with_the_complete_schema() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let path = tmp.path().join("empty.parquet");
    write_table::<serde_json::Value>(&path, funding_schema(), &[]).await?;
    let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(
        std::fs::File::open(path)?,
    )?;
    assert_eq!(reader.schema().as_ref(), &funding_schema());
    assert_eq!(reader.build()?.count(), 0);
    Ok(())
}

#[test]
fn top_level_financial_names_produce_only_funding_scoped_observations() -> anyhow::Result<()> {
    let source = json!({
        "id": 42, "category": {"id": 3, "name": "Donations"},
        "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "fields": [
            {"name": "DonorName", "value": "Top donor"},
            {"name": "PayerName", "value": "Top payer"},
            {"name": "Value", "value": "10"}
        ]}]
    });
    let evidence = replay_declaration(source, chrono::Utc::now())?;
    let cleaned = CleanedDeclarations::from_captures(&[CapturedMemberDeclarations {
        member: MemberId::new(4613)?,
        declarations: vec![evidence],
    }])?;
    assert_eq!(cleaned.summary().funding_entries, 1);
    assert_eq!(cleaned.summary().funders, 2);
    assert!(
        cleaned
            .funders
            .iter()
            .all(|funder| matches!(funder.source, FunderObservationSource::FundingEntry { .. }))
    );
    Ok(())
}

#[test]
fn public_addresses_stay_in_their_role_and_source_scope() -> anyhow::Result<()> {
    let source = json!({"id":42,"category":{"id":3,"name":"Donations"},"versions":[{"register":{"id":820,"publishedDate":"2026-09-07"},"fields":[
        {"name":"DonorName","value":"Root donor"},{"name":"DonorPublicAddress","value":" 1 ROOT Road "},
        {"name":"PayerName","value":"Root payer"},{"name":"PayerPublicAddress","value":"Private address"},
        {"name":"UltimatePayerName","value":"Root ultimate"},{"name":"UltimatePayerAddress","value":""},
        {"name":"Donors","values":[[{"name":"Name","value":"Nested donor"},{"name":"PublicAddress","value":"2 Nested Road"}],[{"name":"Name","value":"Other donor"}]]}
    ]}]});
    let evidence = replay_declaration(source, chrono::Utc::now())?;
    let cleaned = CleanedDeclarations::from_captures(&[CapturedMemberDeclarations {
        member: MemberId::new(4613)?,
        declarations: vec![evidence],
    }])?;
    assert_eq!(cleaned.funders.len(), 5);
    let address = |name: &str| {
        &cleaned
            .funders
            .iter()
            .find(|f| f.name.name_raw.as_deref() == Some(name))
            .unwrap()
            .address
    };
    assert_eq!(
        address("Root donor").normalized.as_deref(),
        Some("1 root road")
    );
    assert_eq!(
        address("Root donor").source_field.as_deref(),
        Some("DonorPublicAddress")
    );
    assert_eq!(
        address("Root payer").raw.as_deref(),
        Some("Private address")
    );
    assert!(address("Root payer").normalized.is_none());
    assert_eq!(address("Root ultimate").raw.as_deref(), Some(""));
    assert!(address("Root ultimate").normalized.is_none());
    assert_eq!(
        address("Nested donor").normalized.as_deref(),
        Some("2 nested road")
    );
    assert_eq!(
        address("Nested donor").source_field.as_deref(),
        Some("PublicAddress")
    );
    assert!(address("Other donor").raw.is_none());
    assert_eq!(cleaned.funding_entries.len(), 2);
    Ok(())
}

#[tokio::test]
async fn raw_source_replays_addresses_without_projected_address_columns() -> anyhow::Result<()> {
    use crate::domain::repositories::{
        declaration_cleaning::DeclarationCleaningStorage, entity_ingestion::EntityIngestionStorage,
    };
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let member = MemberId::new(4613)?;
    let source = json!({"id":42,"category":{"id":3,"name":"Donations"},"versions":[{"register":{"id":820,"publishedDate":"2026-09-07"},"fields":[{"name":"DonorName","value":"Donor"},{"name":"DonorPublicAddress","value":"1 Road"},{"name":"Value","value":"10"}]}]});
    let evidence = replay_declaration(source, chrono::Utc::now())?;
    storage
        .write_raw_declarations(member, &[evidence.declaration().clone()])
        .await?;
    let captures = storage.read_captured_declarations().await?;
    let cleaned = CleanedDeclarations::from_captures(&captures)?;
    assert_eq!(
        cleaned.funders[0].address.normalized.as_deref(),
        Some("1 road")
    );
    assert_eq!(cleaned.funding_entries.len(), 1);
    Ok(())
}

#[tokio::test]
async fn source_partition_rejects_invalid_filenames_and_legacy_empty_identity() -> anyhow::Result<()>
{
    for filename in [
        "0",
        "-1",
        "2147483648",
        "4294967296",
        "04613",
        "+4613",
        "00000000-0000-0000-0000-000000000001",
    ] {
        let tmp = TempDir::new()?;
        let path = tmp.path().join(format!("{filename}.parquet"));
        write_table::<serde_json::Value>(&path, super::declarations_schema(), &[]).await?;
        let error = super::read_partition(&path)
            .err()
            .expect("invalid source filename");
        assert!(
            error.to_string().contains("capture a new ingestion run"),
            "{error}"
        );
    }
    let tmp = TempDir::new()?;
    let path = tmp.path().join("4613.parquet");
    let mut fields = super::declarations_schema().fields().to_vec();
    fields.push(std::sync::Arc::new(arrow::datatypes::Field::new(
        "member_id",
        arrow::datatypes::DataType::Utf8,
        false,
    )));
    write_table::<serde_json::Value>(&path, arrow::datatypes::Schema::new(fields), &[]).await?;
    assert!(
        super::read_partition(&path)
            .err()
            .unwrap()
            .to_string()
            .contains("capture a new ingestion run")
    );
    Ok(())
}

#[tokio::test]
async fn source_partition_roundtrips_and_rejects_invalid_or_mixed_row_identities()
-> anyhow::Result<()> {
    use crate::domain::repositories::entity_ingestion::EntityIngestionStorage;
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let member = MemberId::new(4613)?;
    let evidence = replay_declaration(
        json!({"id":42,"category":{"id":3,"name":"Donations"},"versions":[{"register":{"id":820,"publishedDate":"2026-09-07"},"fields":[{"name":"DonorName","value":"Donor"},{"name":"Value","value":"10"}]}]}),
        chrono::Utc::now(),
    )?;
    storage
        .write_raw_declarations(member, &[evidence.declaration().clone()])
        .await?;
    let path = storage.raw_path().join("declarations/4613.parquet");
    let capture = super::read_partition(&path)?;
    assert_eq!(capture.member, member);
    assert_eq!(capture.declarations.len(), 1);
    assert_eq!(capture.declarations[0].declaration().id().value(), 42);
    let mut json = arrow::json::ArrayWriter::new(Vec::new());
    let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(
        std::fs::File::open(&path)?,
    )?
    .build()?;
    for batch in reader {
        json.write(&batch?)?;
    }
    json.finish()?;
    let original: Vec<serde_json::Value> = serde_json::from_slice(&json.into_inner())?;
    assert!(original[0].get("member_id").is_none());
    for (field, value, mixed) in [
        ("parliament_member_id", 0, false),
        ("parliament_member_id", u32::MAX, false),
        ("parliament_member_id", 5030, false),
        ("parliament_member_id", 5030, true),
        ("declaration_id", 0, false),
        ("parent_declaration_id", 0, false),
    ] {
        let mut rows = original.clone();
        let mut invalid = original[0].clone();
        invalid[field] = json!(value);
        if mixed {
            rows.push(invalid);
        } else {
            rows[0] = invalid;
        }
        std::fs::remove_file(&path)?;
        let schema = std::sync::Arc::new(super::declarations_schema());
        let encoded = rows
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        let batches =
            arrow::json::ReaderBuilder::new(schema.clone()).build(std::io::Cursor::new(encoded))?;
        let mut writer =
            parquet::arrow::ArrowWriter::try_new(std::fs::File::create(&path)?, schema, None)?;
        for batch in batches {
            writer.write(&batch?)?;
        }
        writer.close()?;
        assert!(
            super::read_partition(&path).is_err(),
            "accepted invalid {field}={value}, mixed={mixed}"
        );
    }
    Ok(())
}
