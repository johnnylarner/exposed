use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

use super::{ExposedDataPipeline, funding_schema, replay_declaration, write_table};
use crate::domain::models::{
    declaration_cleaning::{
        CapturedMemberDeclarations, CleanedDeclarations, FunderObservationSource,
    },
    declaration_ingestion::MemberAsId,
    entity_ingestion::IngestionKey,
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
    let cleaned = CleanedDeclarations::from_captures(&[CapturedMemberDeclarations::Populated {
        member: MemberAsId::new(Uuid::from_u128(1), 4613)?,
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
    let cleaned = CleanedDeclarations::from_captures(&[CapturedMemberDeclarations::Populated {
        member: MemberAsId::new(Uuid::from_u128(1), 4613)?,
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
async fn old_raw_projection_replays_addresses_without_new_raw_columns() -> anyhow::Result<()> {
    use crate::domain::repositories::{
        declaration_cleaning::DeclarationCleaningStorage, entity_ingestion::EntityIngestionStorage,
    };
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let member = MemberAsId::new(Uuid::from_u128(1), 4613)?;
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
