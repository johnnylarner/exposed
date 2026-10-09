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
struct TestScorer(f64);

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
                        probability: self.0,
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
    DeclarationResolverService::new(storage.clone(), TestScorer(0.5), 100)
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

#[tokio::test]
async fn supporting_name_resolution_loads_and_rejects_unknown_labels() -> anyhow::Result<()> {
    for (probability, expected_basis) in [
        (0.5, "extracted_name_link"),
        (0.9999, "statistical_and_supporting_name_link"),
    ] {
        let temporary = tempfile::tempdir()?;
        let storage = ExposedDataPipeline::new_with_ingestion_key(
            &temporary.path().join("data"),
            IngestionKey::default(),
        )?;
        let member = MemberAsId::new(Uuid::from_u128(1), 4613)?;
        let fetched_at = Utc::now();
        let bank_address = "1 Centenary Square Birmingham B1 1HQ";
        let declarations = [
            ("Unite Union", "Trade Union", ""),
            ("Unite West Midlands", "Trade Union", ""),
            ("HSBC UK (Ian Stuart, CEO)", "Company", bank_address),
            ("HSBC UK Bank plc", "Company", bank_address),
            ("HSBC UK Bank plc", "Company", bank_address),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (name, kind, address))| {
            super::super::declaration_source::replay_declaration(json!({
                "id": index + 1,
                "category": {"id": 3, "name": "Donations"},
                "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "fields": [
                    {"name": "DonorName", "value": name},
                    {"name": "DonorStatus", "value": kind},
                    {"name": "DonorPublicAddress", "value": address},
                    {"name": "Value", "value": "20"}
                ]}]
            }), fetched_at).map(|evidence| evidence.declaration().clone())
        })
        .collect::<Result<Vec<CapturedDeclaration>, _>>()?;
        storage
            .write_raw_declarations(member, &declarations)
            .await?;
        DeclarationCleanerService::new(storage.clone())
            .clean_declarations()
            .await?;
        DeclarationResolverService::new(storage.clone(), TestScorer(probability), 100)
            .resolve_declarations()
            .await?;

        let resolved = storage.resolved_declarations_path();
        let identity_path = resolved.join("observation_resolution.parquet");
        let pair_path = resolved.join("pair_decisions.parquet");
        let (identities, _) = read_table::<serde_json::Value>(
            &identity_path,
            &declaration_resolution::observation_schema(),
        )?;
        let (pairs, _) =
            read_table::<serde_json::Value>(&pair_path, &declaration_resolution::pair_schema())?;
        for basis in ["trade_union_family_link", expected_basis] {
            assert!(
                identities.iter().any(|row| row["identity_basis"] == basis),
                "missing {basis}"
            );
        }
        for reason in ["trade_union_family", "extracted_name_evidence"] {
            assert!(
                pairs.iter().any(|row| row["reason"] == reason),
                "missing {reason}"
            );
        }
        let load = storage.read_declaration_load().await?;
        assert_eq!(load.funders.len(), 2);
        assert_eq!(load.funding_entries.len(), 5);
        for ids in [&[1, 2][..], &[3, 4, 5][..]] {
            let identities = load
                .funding_entries
                .iter()
                .filter(|entry| ids.contains(&entry.source_declaration_id))
                .map(|entry| entry.identity_id.as_ref().expect("resolved funder"))
                .collect::<BTreeSet<_>>();
            assert_eq!(identities.len(), 1);
        }

        let mut malformed = identities;
        malformed[0]["identity_basis"] = json!("unknown_resolution_label");
        fs::remove_file(&identity_path)?;
        declaration_cleaning::write_table(
            &identity_path,
            declaration_resolution::observation_schema(),
            &malformed,
        )
        .await?;
        let error = storage
            .read_declaration_load()
            .await
            .err()
            .expect("unknown label rejected");
        assert!(
            error
                .to_string()
                .contains("invalid or duplicate resolved observation identity"),
            "{error}"
        );
    }
    Ok(())
}

async fn database_snapshot(pool: &sqlx::PgPool) -> sqlx::Result<serde_json::Value> {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
            'members', (SELECT jsonb_agg(to_jsonb(row) ORDER BY id) FROM exposed.members AS row),
            'declarations', (SELECT jsonb_agg(to_jsonb(row) ORDER BY id) FROM exposed.declarations AS row),
            'funders', (SELECT jsonb_agg(to_jsonb(row) ORDER BY id) FROM exposed.funders AS row),
            'aliases', (SELECT jsonb_agg(to_jsonb(row) ORDER BY funder_id, funder_alias) FROM exposed.funder_aliases AS row),
            'entries', (SELECT jsonb_agg(to_jsonb(row) ORDER BY id) FROM exposed.funding_entries AS row),
            'runs', (SELECT jsonb_agg(to_jsonb(row) ORDER BY ingestion_key) FROM exposed.declaration_load_runs AS row)
        ) AS "snapshot!""#
    )
    .fetch_one(pool)
    .await
}

#[sqlx::test(migrations = "../db/migrations")]
async fn refresh_replaces_funders_and_preserves_untouched_declarations(
    pool: sqlx::PgPool,
) -> anyhow::Result<()> {
    let (_temporary, mut load) = resolved_test_run().await?;
    sqlx::query!(
        "INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) VALUES ($1, 4613, 'Test member', 1, 'Test party', 1, 'Test constituency', true)",
        Uuid::from_u128(1)
    )
    .execute(&pool)
    .await?;
    let repository = ExposedDatabase::from(pool.clone());
    repository.load_declarations(&load).await?;
    let shared = sqlx::query!("SELECT id FROM exposed.funders WHERE company_number = '12345678'")
        .fetch_one(&pool)
        .await?;
    sqlx::query!(
        "INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date) SELECT 5, member_id, category_id, category_name, fetched_at, registration_date FROM exposed.declarations WHERE source_declaration_id = 4"
    )
    .execute(&pool)
    .await?;
    sqlx::query!(
        "INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, source_funding_entry_id) VALUES (5, $1, 'untouched-shared-company')",
        shared.id
    )
    .execute(&pool)
    .await?;
    let original = database_snapshot(&pool).await?;
    let old_local = sqlx::query!(
        "SELECT funder_id FROM exposed.funding_entries WHERE source_declaration_id = 1"
    )
    .fetch_one(&pool)
    .await?
    .funder_id
    .unwrap();
    assert!(matches!(
        repository.load_declarations(&load).await?.outcome,
        DeclarationLoadOutcome::AlreadyLoaded
    ));
    assert_eq!(database_snapshot(&pool).await?, original);

    sqlx::query!("INSERT INTO exposed.funders (funder_name) VALUES ('Legacy orphan')")
        .execute(&pool)
        .await?;
    sqlx::query!(
        "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) SELECT id, 'Legacy alias' FROM exposed.funders WHERE funder_name = 'Legacy orphan'"
    )
    .execute(&pool)
    .await?;
    let before_refresh = database_snapshot(&pool).await?;
    load.ingestion_key = IngestionKey::default();
    load.fingerprint = "refreshed artifacts".to_owned();
    load.declarations
        .retain(|declaration| matches!(declaration.source_declaration_id, 1 | 4));
    load.funding_entries
        .retain(|entry| matches!(entry.source_declaration_id, 1 | 4));
    let retained = load
        .funding_entries
        .iter()
        .filter_map(|entry| entry.identity_id.clone())
        .collect::<BTreeSet<_>>();
    load.funders
        .retain(|funder| retained.contains(&funder.identity_id));
    for funder in &mut load.funders {
        if funder.company_number.is_some() {
            funder.name = "Current company name".to_owned();
            funder.aliases = vec!["Current company alias".to_owned()];
            funder.kind = None;
            funder.company_number = None;
        } else {
            let new_identity = format!("run-local:{}:replacement", load.ingestion_key.uuid());
            for entry in &mut load.funding_entries {
                if entry.identity_id.as_deref() == Some(funder.identity_id.as_str()) {
                    entry.identity_id = Some(new_identity.clone());
                }
            }
            funder.identity_id = new_identity;
        }
    }
    assert!(repository.load_declarations(&load).await.is_err());
    assert_eq!(database_snapshot(&pool).await?, before_refresh);
    for declaration in &mut load.declarations {
        declaration.fetched_at += chrono::Duration::seconds(1);
    }
    repository.load_declarations(&load).await?;
    let refreshed = database_snapshot(&pool).await?;
    assert_eq!(refreshed["members"], original["members"]);
    for table in ["declarations", "entries"] {
        let untouched = |snapshot: &serde_json::Value| {
            snapshot[table]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| matches!(row["source_declaration_id"].as_u64(), Some(2 | 3 | 5)))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(untouched(&refreshed), untouched(&original));
    }
    let funder = sqlx::query!(
        "SELECT funder_name, funder_kind, company_number FROM exposed.funders WHERE id = $1",
        shared.id
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(funder.funder_name, "Current company name");
    assert_eq!(funder.funder_kind, None);
    assert_eq!(funder.company_number, None);
    let aliases = sqlx::query_scalar!(
        "SELECT funder_alias FROM exposed.funder_aliases WHERE funder_id = $1 ORDER BY funder_alias",
        shared.id
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(aliases, ["Current company alias", "Current company name"]);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT funder_id FROM exposed.funding_entries WHERE source_declaration_id = 4"
        )
        .fetch_one(&pool)
        .await?,
        Some(shared.id)
    );
    assert!(
        sqlx::query!("SELECT id FROM exposed.funders WHERE id = $1", old_local)
            .fetch_optional(&pool)
            .await?
            .is_none()
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM exposed.funders AS funder WHERE NOT EXISTS (SELECT 1 FROM exposed.funding_entries AS entry WHERE entry.funder_id = funder.id)"
        )
        .fetch_one(&pool)
        .await?,
        Some(0)
    );
    assert!(matches!(
        repository.load_declarations(&load).await?.outcome,
        DeclarationLoadOutcome::AlreadyLoaded
    ));
    assert_eq!(database_snapshot(&pool).await?, refreshed);

    let removed = sqlx::query_scalar!(
        "SELECT funder_id FROM exposed.funding_entries WHERE source_declaration_id = 1"
    )
    .fetch_one(&pool)
    .await?
    .unwrap();
    load.ingestion_key = IngestionKey::default();
    load.fingerprint = "declaration without funding".to_owned();
    load.declarations
        .retain(|declaration| declaration.source_declaration_id == 1);
    load.declarations[0].fetched_at += chrono::Duration::seconds(1);
    load.funding_entries.clear();
    load.funders.clear();
    repository.load_declarations(&load).await?;
    assert!(
        sqlx::query!("SELECT id FROM exposed.funders WHERE id = $1", removed)
            .fetch_optional(&pool)
            .await?
            .is_none()
    );
    assert!(
        sqlx::query!(
            "SELECT funder_alias FROM exposed.funder_aliases WHERE funder_id = $1",
            removed
        )
        .fetch_all(&pool)
        .await?
        .is_empty()
    );
    let without_funding = database_snapshot(&pool).await?;
    for table in ["declarations", "entries"] {
        let untouched = |snapshot: &serde_json::Value| {
            snapshot[table]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["source_declaration_id"].as_u64() != Some(1))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(untouched(&without_funding), untouched(&refreshed));
    }
    assert!(
        without_funding["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["source_declaration_id"] != 1)
    );
    Ok(())
}
