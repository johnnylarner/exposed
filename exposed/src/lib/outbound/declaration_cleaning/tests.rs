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
