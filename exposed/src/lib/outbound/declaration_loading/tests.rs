use super::*;
use crate::domain::{
    models::{
        declaration_ingestion::{CapturedDeclaration, MemberAsId},
        declaration_resolution::{ScoredPair, ScoredPairs, ScoringInput},
        entity_ingestion::{EntityIngestionError, IngestionKey},
    },
    repositories::{
        declaration_loading::DeclarationLoadStorage, declaration_resolution::FunderScorer,
        entity_ingestion::EntityIngestionStorage,
    },
    services::{
        declaration_cleaning::DeclarationCleanerService,
        declaration_resolution::DeclarationResolverService,
    },
};
use chrono::Utc;
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn canonical_funder_name_prefers_the_most_reported_source_spelling() {
    assert_eq!(
        canonical_name([
            "East Midlands Unite the Union",
            "Unite",
            "Unite the Union",
            "Unite the Union",
        ]),
        Some("Unite the Union")
    );
}

#[derive(Clone, Copy)]
struct TestScorer;

impl FunderScorer for TestScorer {
    async fn score(&self, input: &ScoringInput) -> Result<ScoredPairs, EntityIngestionError> {
        let pairs = input
            .rows
            .iter()
            .enumerate()
            .flat_map(|(index, left)| {
                input.rows[index + 1..].iter().map(move |right| {
                    let (left, right) = if left.key < right.key {
                        (left, right)
                    } else {
                        (right, left)
                    };
                    ScoredPair {
                        left: left.key.clone(),
                        right: right.key.clone(),
                        probability: 0.5,
                        name_level: 0,
                        address_level: -1,
                    }
                })
            })
            .collect();
        ScoredPairs::checked(pairs, json!({"calibrated": false}), input)
    }
}

async fn resolved_test_run() -> anyhow::Result<(TempDir, DeclarationLoad)> {
    let temporary = tempfile::tempdir()?;
    let key = IngestionKey::default();
    let root = temporary.path().join("data");
    let storage = ExposedDataPipeline::new_with_ingestion_key(&root, key)?;
    let member = MemberAsId::new(Uuid::from_u128(1), 4613)?;
    let fetched_at = Utc::now();
    let declarations = [
        json!({
            "id": 1,
            "category": {"id": 3, "name": "Donations"},
            "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "fields": [
                {"name": "PayerName", "value": "Parent payer"},
                {"name": "Value", "value": "20"}
            ]}]
        }),
        json!({
            "id": 2,
            "parentInterestId": 1,
            "category": {"id": 3, "name": "Donations"},
            "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "fields": [
                {"name": "UltimatePayerName", "value": "Explicit ultimate payer"},
                {"name": "DonorName", "value": "Child donor"},
                {"name": "Value", "value": "30"}
            ]}]
        }),
        json!({
            "id": 3,
            "category": {"id": 3, "name": "Donations"},
            "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "fields": [
                {"name": "DonorName", "value": "Shared funder"},
                {"name": "DonorStatus", "value": "Unincorporated association"},
                {"name": "Value", "value": "40"}
            ]}]
        }),
        json!({
            "id": 4,
            "category": {"id": 3, "name": "Donations"},
            "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "fields": [
                {"name": "DonorName", "value": "Shared funder"},
                {"name": "DonorStatus", "value": "Company"},
                {"name": "DonorCompanyIdentifier", "value": "12345678"},
                {"name": "Value", "value": "50"}
            ]}]
        }),
    ]
    .into_iter()
    .map(|source| {
        super::super::declaration_source::replay_declaration(source, fetched_at)
            .map(|evidence| evidence.declaration().clone())
    })
    .collect::<Result<Vec<CapturedDeclaration>, _>>()?;
    storage
        .write_raw_declarations(member, &declarations)
        .await?;
    DeclarationCleanerService::new(storage.clone())
        .clean_declarations()
        .await?;
    DeclarationResolverService::new(storage.clone(), TestScorer, 100)
        .resolve_declarations()
        .await?;
    let load = storage.read_declaration_load().await?;
    Ok((temporary, load))
}

#[tokio::test]
async fn explicit_ultimate_payer_on_child_declaration_is_loadable() -> anyhow::Result<()> {
    let (_temporary, load) = resolved_test_run().await?;
    let child_attribution = load
        .funding_entries
        .iter()
        .find(|entry| entry.attribution_basis.as_deref() == Some("explicit_ultimate_payer"))
        .expect("the child payment selects its explicit ultimate payer");

    assert_eq!(child_attribution.source_declaration_id, 2);
    assert_eq!(child_attribution.selected_parent_declaration_id, None);
    Ok(())
}

#[tokio::test]
async fn company_number_uses_the_matching_company_kind() -> anyhow::Result<()> {
    let (_temporary, load) = resolved_test_run().await?;
    let company = load
        .funders
        .iter()
        .find(|funder| funder.company_number.as_deref() == Some("12345678"))
        .expect("the resolved identity retains its reported company number");

    assert_eq!(company.kind.as_deref(), Some("Company"));
    Ok(())
}
