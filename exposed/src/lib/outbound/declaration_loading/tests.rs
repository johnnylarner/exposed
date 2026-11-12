use super::*;
use crate::domain::{
    models::{
        declaration_ingestion::CapturedDeclaration,
        declaration_resolution::{ScoredPair, ScoredPairs, ScoringInput, UnavailableReason},
        entity_ingestion::{EntityIngestionError, IngestionKey},
        parliament_member::MemberId,
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
    fn score(
        &self,
        input: &ScoringInput,
    ) -> impl Future<Output = Result<ScoredPairs, EntityIngestionError>> {
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
        std::future::ready(ScoredPairs::checked(
            pairs,
            json!({"calibrated": false}),
            input,
        ))
    }
}

async fn resolved_test_run() -> anyhow::Result<(TempDir, DeclarationLoad)> {
    let temporary = tempfile::tempdir()?;
    let key = IngestionKey::default();
    let root = temporary.path().join("data");
    let storage = ExposedDataPipeline::new_with_ingestion_key(&root, key)?;
    let member = MemberId::new(4613)?;
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
        json!({
            "id": 6,
            "category": {"id": 3, "name": "Donations"},
            "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "fields": [
                {"name": "Value", "value": "20"}
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
        .find(|entry| {
            matches!(
                entry.attribution_basis,
                Some(AttributionBasis::ExplicitUltimatePayer)
            )
        })
        .expect("the child payment selects its explicit ultimate payer");

    assert_eq!(child_attribution.source_declaration_id.value(), 2);
    assert_eq!(child_attribution.selected_parent_declaration_id, None);
    Ok(())
}

#[tokio::test]
async fn unavailable_attribution_roundtrips_through_parquet() -> anyhow::Result<()> {
    let (_temporary, load) = resolved_test_run().await?;
    let entry = load
        .funding_entries
        .iter()
        .find(|entry| entry.source_declaration_id.value() == 6)
        .expect("the unattributed payment remains in the load");
    assert!(matches!(
        entry.unavailable_reason,
        Some(UnavailableReason::NoSupportedAttribution)
    ));
    assert!(entry.identity_id.is_none());
    assert!(entry.attribution_basis.is_none());
    assert!(entry.selected_observation_id.is_none());
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
        let member = MemberId::new(4613)?;
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
                .filter(|entry| ids.contains(&entry.source_declaration_id.value()))
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
                .contains("unknown variant `unknown_resolution_label`"),
            "{error}"
        );
    }
    Ok(())
}

async fn database_snapshot(pool: &sqlx::PgPool) -> sqlx::Result<serde_json::Value> {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
            'commons_profiles', (SELECT jsonb_agg(to_jsonb(row) ORDER BY member_id) FROM exposed.member_profiles AS row WHERE chamber = 'commons'),
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
        "WITH input (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) AS (
    VALUES ($1::uuid, 4613, 'Test member', 1, 'Test party', 1, 'Test constituency', true)
), identities AS (
    INSERT INTO exposed.members (id, parliament_member_id)
    SELECT input.id::uuid, input.parliament_member_id::integer FROM input RETURNING id, parliament_member_id
)
INSERT INTO exposed.member_profiles (member_id, chamber, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons, created_at, updated_at)
SELECT identities.id, 'commons', input.name, input.party_id, input.party_name, input.latest_house::smallint, input.latest_membership_from, input.is_current_commons, now(), now()
FROM input JOIN identities ON identities.parliament_member_id = input.parliament_member_id",
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
    load.fingerprint = "changed artifacts under the same key".to_owned();
    assert!(repository.load_declarations(&load).await.is_err());
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
        .retain(|declaration| matches!(declaration.source_declaration_id.value(), 1 | 4));
    load.funding_entries
        .retain(|entry| matches!(entry.source_declaration_id.value(), 1 | 4));
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
    for declaration in &mut load.declarations {
        declaration.category_name = "Updated category".to_owned();
        declaration.fetched_at +=
            chrono::Duration::seconds(if declaration.source_declaration_id.value() == 1 {
                1
            } else {
                -1
            });
    }
    let error = repository.load_declarations(&load).await.unwrap_err();
    assert!(error.to_string().contains("declaration 4"), "{error}");
    assert!(error.to_string().contains("newer capture time"), "{error}");
    assert_eq!(database_snapshot(&pool).await?, before_refresh);
    for declaration in &mut load.declarations {
        declaration.fetched_at +=
            chrono::Duration::seconds(if declaration.source_declaration_id.value() == 1 {
                -1
            } else {
                2
            });
    }
    repository.load_declarations(&load).await?;
    let refreshed = database_snapshot(&pool).await?;
    assert_eq!(refreshed["members"], original["members"]);
    for declaration in refreshed["declarations"].as_array().unwrap() {
        if matches!(declaration["source_declaration_id"].as_u64(), Some(1 | 4)) {
            assert_eq!(declaration["category_name"], "Updated category");
        }
    }
    for table in ["declarations", "entries"] {
        let untouched = |snapshot: &serde_json::Value| {
            snapshot[table]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| matches!(row["source_declaration_id"].as_u64(), Some(2 | 3 | 5 | 6)))
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
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT funder_id FROM exposed.funding_entries WHERE source_declaration_id = 1"
        )
        .fetch_one(&pool)
        .await?,
        Some(old_local)
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
        .retain(|declaration| declaration.source_declaration_id.value() == 1);
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

#[sqlx::test(migrations = "../db/migrations")]
async fn loader_groups_retained_identities_and_exact_retry_refreshes_without_replay(
    pool: sqlx::PgPool,
) -> anyhow::Result<()> {
    let (_temporary, load) = resolved_test_run().await?;
    sqlx::query!("WITH input (parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) AS (
    VALUES (4613, 'Test member', 1, 'Test party', 1, 'Test constituency', true)
), identities AS (
    INSERT INTO exposed.members (parliament_member_id)
    SELECT input.parliament_member_id::integer FROM input RETURNING id, parliament_member_id
)
INSERT INTO exposed.member_profiles (member_id, chamber, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons, created_at, updated_at)
SELECT identities.id, 'commons', input.name, input.party_id, input.party_name, input.latest_house::smallint, input.latest_membership_from, input.is_current_commons, now(), now()
FROM input JOIN identities ON identities.parliament_member_id = input.parliament_member_id").execute(&pool).await?;
    let old_id = Uuid::from_u128(1);
    sqlx::query!(
        "INSERT INTO exposed.funders (id, funder_name) VALUES ($1, 'Retained old source')",
        old_id
    )
    .execute(&pool)
    .await?;
    sqlx::query!(
        "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2)",
        old_id,
        load.funders[0].name
    )
    .execute(&pool)
    .await?;
    sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at) SELECT 9000, member_id, 1, 'Retained', now() FROM exposed.member_profiles WHERE chamber = 'commons' LIMIT 1").execute(&pool).await?;
    sqlx::query!("INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency) VALUES (9000, $1, 7, 'GBP')", old_id).execute(&pool).await?;
    let repository = ExposedDatabase::from(pool.clone());
    repository.load_declarations(&load).await?;
    let new_id = sqlx::query_scalar!(
        "SELECT id FROM exposed.funders WHERE resolution_identity_id = $1",
        load.funders[0].identity_id
    )
    .fetch_one(&pool)
    .await?;
    let group = sqlx::query_scalar!(
        "SELECT group_id FROM exposed.funder_alias_groups WHERE funder_id = $1",
        new_id
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(group, old_id);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.funder_alias_groups")
            .fetch_one(&pool)
            .await?,
        Some(i64::try_from(load.funders.len() + 1)?)
    );

    // The retry must use current stored inputs, including changes outside this ingestion run.
    sqlx::query!(
        "DELETE FROM exposed.funder_aliases WHERE funder_id = $1",
        old_id
    )
    .execute(&pool)
    .await?;
    let before = database_snapshot(&pool).await?;
    assert_eq!(
        repository.load_declarations(&load).await?.outcome,
        DeclarationLoadOutcome::AlreadyLoaded
    );
    assert_eq!(database_snapshot(&pool).await?, before);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT group_id FROM exposed.funder_alias_groups WHERE funder_id = $1",
            new_id
        )
        .fetch_one(&pool)
        .await?,
        new_id
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT group_id FROM exposed.funder_alias_groups WHERE funder_id = $1",
            old_id
        )
        .fetch_one(&pool)
        .await?,
        old_id
    );
    Ok(())
}

async fn seed_load_member(pool: &sqlx::PgPool) -> sqlx::Result<()> {
    sqlx::query!("WITH input (parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) AS (
    VALUES (4613, 'Test member', 1, 'Test party', 1, 'Test constituency', true)
), identities AS (
    INSERT INTO exposed.members (parliament_member_id)
    SELECT input.parliament_member_id::integer FROM input RETURNING id, parliament_member_id
)
INSERT INTO exposed.member_profiles (member_id, chamber, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons, created_at, updated_at)
SELECT identities.id, 'commons', input.name, input.party_id, input.party_name, input.latest_house::smallint, input.latest_membership_from, input.is_current_commons, now(), now()
FROM input JOIN identities ON identities.parliament_member_id = input.parliament_member_id").execute(pool).await?;
    Ok(())
}

fn fresh_identity_keys(load: &mut DeclarationLoad) {
    load.ingestion_key = IngestionKey::default();
    load.fingerprint = load.ingestion_key.to_string();
    for funder in &mut load.funders {
        if funder.company_number.is_some() {
            continue;
        }
        let old = std::mem::replace(&mut funder.identity_id, Uuid::new_v4().to_string());
        for entry in &mut load.funding_entries {
            if entry.identity_id.as_deref() == Some(old.as_str()) {
                entry.identity_id = Some(funder.identity_id.clone());
            }
        }
    }
}

async fn profile_ids(pool: &sqlx::PgPool) -> sqlx::Result<BTreeMap<i32, (Uuid, Uuid)>> {
    let rows = sqlx::query!(
        r#"SELECT entry.source_declaration_id, entry.funder_id AS "funder_id!", grouping.group_id
        FROM exposed.funding_entries AS entry JOIN exposed.funder_alias_groups AS grouping ON grouping.funder_id = entry.funder_id
        ORDER BY entry.source_declaration_id"#
    ).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.source_declaration_id, (row.funder_id, row.group_id)))
        .collect())
}

async fn funding_values(pool: &sqlx::PgPool) -> sqlx::Result<serde_json::Value> {
    sqlx::query_scalar!(
        r#"SELECT jsonb_agg(to_jsonb(entry) - 'id' - 'created_at' - 'updated_at' ORDER BY source_funding_entry_id) AS "values!" FROM exposed.funding_entries AS entry"#
    ).fetch_one(pool).await
}

#[sqlx::test(migrations = "../db/migrations")]
async fn fresh_resolution_preserves_funder_and_profile_ids_and_prior_retry(
    pool: sqlx::PgPool,
) -> anyhow::Result<()> {
    seed_load_member(&pool).await?;
    let (_old_artifacts, old) = resolved_test_run().await?;
    let repository = ExposedDatabase::from(pool.clone());
    repository.load_declarations(&old).await?;
    let original_ids = profile_ids(&pool).await?;
    let original_values = funding_values(&pool).await?;
    let (_fresh_artifacts, mut fresh) = resolved_test_run().await?;
    assert_ne!(old.ingestion_key.uuid(), fresh.ingestion_key.uuid());
    for (old_row, fresh_row) in old.declarations.iter().zip(&fresh.declarations) {
        assert!(fresh_row.fetched_at > old_row.fetched_at);
    }
    repository.load_declarations(&fresh).await?;
    assert_eq!(profile_ids(&pool).await?, original_ids);
    assert_eq!(funding_values(&pool).await?, original_values);
    let before_retry = database_snapshot(&pool).await?;
    assert!(matches!(
        repository.load_declarations(&old).await?.outcome,
        DeclarationLoadOutcome::AlreadyLoaded
    ));
    assert_eq!(database_snapshot(&pool).await?, before_retry);

    fresh_identity_keys(&mut fresh);
    fresh
        .funding_entries
        .iter_mut()
        .find(|entry| entry.source_declaration_id.value() == 1)
        .unwrap()
        .amount = Some("123.4567890123456789".parse()?);
    for funder in &mut fresh.funders {
        funder.aliases.push(funder.name.clone());
        funder.aliases.reverse();
    }
    repository.load_declarations(&fresh).await?;
    assert_eq!(profile_ids(&pool).await?, original_ids);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT amount::text FROM exposed.funding_entries WHERE source_declaration_id = 1"
        )
        .fetch_one(&pool)
        .await?
        .as_deref(),
        Some("123.4567890123456789")
    );

    fresh_identity_keys(&mut fresh);
    let changed = fresh
        .funding_entries
        .iter()
        .find(|entry| entry.source_declaration_id.value() == 2)
        .unwrap()
        .identity_id
        .as_ref()
        .unwrap()
        .clone();
    fresh
        .funders
        .iter_mut()
        .find(|funder| funder.identity_id == changed)
        .unwrap()
        .name = "Changed unrelated funder".to_owned();
    repository.load_declarations(&fresh).await?;
    let changed_ids = profile_ids(&pool).await?;
    for declaration in [1, 3, 4] {
        assert_eq!(changed_ids[&declaration], original_ids[&declaration]);
    }
    assert_ne!(changed_ids[&2].0, original_ids[&2].0);
    Ok(())
}

fn keep_declarations(load: &mut DeclarationLoad, declarations: &[u32]) {
    load.declarations
        .retain(|row| declarations.contains(&row.source_declaration_id.value()));
    load.funding_entries
        .retain(|row| declarations.contains(&row.source_declaration_id.value()));
    let keys = load
        .funding_entries
        .iter()
        .filter_map(|entry| entry.identity_id.as_deref())
        .collect::<BTreeSet<_>>();
    load.funders
        .retain(|row| keys.contains(row.identity_id.as_str()));
}

fn copy_funder(funder: &LoadFunder, identity_id: String) -> LoadFunder {
    LoadFunder {
        identity_id,
        name: funder.name.clone(),
        aliases: funder.aliases.clone(),
        kind: funder.kind.clone(),
        company_number: funder.company_number.clone(),
    }
}

#[sqlx::test(migrations = "../db/migrations")]
async fn splits_merges_and_same_name_identities_do_not_inherit_ids(
    pool: sqlx::PgPool,
) -> anyhow::Result<()> {
    seed_load_member(&pool).await?;
    let (_artifacts, mut load) = resolved_test_run().await?;
    keep_declarations(&mut load, &[1, 2]);
    let first_key = load.funding_entries[0].identity_id.clone().unwrap();
    load.funders
        .retain(|funder| funder.identity_id == first_key);
    for entry in &mut load.funding_entries {
        entry.identity_id = Some(first_key.clone());
    }
    let repository = ExposedDatabase::from(pool.clone());
    repository.load_declarations(&load).await?;
    let merged = profile_ids(&pool).await?[&1].0;
    assert_eq!(profile_ids(&pool).await?[&2].0, merged);

    fresh_identity_keys(&mut load);
    let split_key = Uuid::new_v4().to_string();
    load.funders
        .push(copy_funder(&load.funders[0], split_key.clone()));
    load.funding_entries[1].identity_id = Some(split_key);
    repository.load_declarations(&load).await?;
    let split = profile_ids(&pool).await?;
    assert_ne!(split[&1].0, merged);
    assert_ne!(split[&2].0, merged);
    assert_ne!(split[&1].0, split[&2].0);

    fresh_identity_keys(&mut load);
    repository.load_declarations(&load).await?;
    assert_eq!(profile_ids(&pool).await?, split);

    fresh_identity_keys(&mut load);
    let merged_key = load.funders[0].identity_id.clone();
    for entry in &mut load.funding_entries {
        entry.identity_id = Some(merged_key.clone());
    }
    load.funders.truncate(1);
    repository.load_declarations(&load).await?;
    let remerged = profile_ids(&pool).await?;
    assert_eq!(remerged[&1].0, remerged[&2].0);
    assert_ne!(remerged[&1].0, split[&1].0);
    assert_ne!(remerged[&1].0, split[&2].0);
    Ok(())
}

#[sqlx::test(migrations = "../db/migrations")]
async fn partial_refresh_refuses_shared_old_identity_but_reuses_complete_identity(
    pool: sqlx::PgPool,
) -> anyhow::Result<()> {
    seed_load_member(&pool).await?;
    let (_artifacts, mut load) = resolved_test_run().await?;
    keep_declarations(&mut load, &[1, 2]);
    let first_key = load.funding_entries[0].identity_id.clone().unwrap();
    load.funders
        .retain(|funder| funder.identity_id == first_key);
    for entry in &mut load.funding_entries {
        entry.identity_id = Some(first_key.clone());
    }
    let repository = ExposedDatabase::from(pool.clone());
    repository.load_declarations(&load).await?;
    let original = profile_ids(&pool).await?[&1].0;
    keep_declarations(&mut load, &[1]);
    fresh_identity_keys(&mut load);
    repository.load_declarations(&load).await?;
    let partial = profile_ids(&pool).await?;
    assert_ne!(partial[&1].0, original);
    assert_eq!(partial[&2].0, original);
    fresh_identity_keys(&mut load);
    repository.load_declarations(&load).await?;
    assert_eq!(profile_ids(&pool).await?, partial);
    Ok(())
}

#[sqlx::test(migrations = "../db/migrations")]
async fn legacy_incomplete_provenance_cannot_reuse_identity(
    pool: sqlx::PgPool,
) -> anyhow::Result<()> {
    seed_load_member(&pool).await?;
    let (_artifacts, mut load) = resolved_test_run().await?;
    let repository = ExposedDatabase::from(pool.clone());
    repository.load_declarations(&load).await?;
    let original = profile_ids(&pool).await?;
    sqlx::query!("UPDATE exposed.funding_entries SET selected_observation_id = NULL WHERE source_declaration_id = 1").execute(&pool).await?;
    fresh_identity_keys(&mut load);
    repository.load_declarations(&load).await?;
    let refreshed = profile_ids(&pool).await?;
    assert_ne!(refreshed[&1].0, original[&1].0);
    for declaration in [2, 3, 4] {
        assert_eq!(refreshed[&declaration], original[&declaration]);
    }
    Ok(())
}

#[sqlx::test(migrations = "../db/migrations")]
async fn exact_keys_take_priority_over_continuity_even_outside_source_candidates(
    pool: sqlx::PgPool,
) -> anyhow::Result<()> {
    seed_load_member(&pool).await?;
    let (_artifacts, mut load) = resolved_test_run().await?;
    keep_declarations(&mut load, &[1, 2]);
    let key1 = load.funding_entries[0].identity_id.clone().unwrap();
    let key2 = load.funding_entries[1].identity_id.clone().unwrap();
    let first = load
        .funders
        .iter()
        .find(|funder| funder.identity_id == key1)
        .unwrap();
    load.funders = vec![
        copy_funder(first, key1.clone()),
        copy_funder(first, key2.clone()),
    ];
    let repository = ExposedDatabase::from(pool.clone());
    repository.load_declarations(&load).await?;
    let original = profile_ids(&pool).await?;
    sqlx::query!("UPDATE exposed.funding_entries SET attribution_basis = 'legacy-unsupported' WHERE source_declaration_id = 2").execute(&pool).await?;
    let key3 = Uuid::new_v4().to_string();
    load.ingestion_key = IngestionKey::default();
    load.funding_entries[0].identity_id = Some(key2.clone());
    load.funding_entries[1].identity_id = Some(key3.clone());
    load.funders = vec![
        copy_funder(&load.funders[0], key2),
        copy_funder(&load.funders[0], key3.clone()),
    ];
    repository.load_declarations(&load).await?;
    let competed = profile_ids(&pool).await?;
    assert_eq!(competed[&1].0, original[&2].0);
    assert_ne!(competed[&2].0, original[&2].0);
    assert_ne!(competed[&2].0, original[&1].0);

    keep_declarations(&mut load, &[1]);
    load.ingestion_key = IngestionKey::default();
    load.funding_entries[0].identity_id = Some(key3.clone());
    load.funders = vec![copy_funder(&load.funders[0], key3)];
    repository.load_declarations(&load).await?;
    let partial = profile_ids(&pool).await?;
    assert_eq!(partial[&1].0, competed[&2].0);
    assert_eq!(partial[&2].0, competed[&2].0);
    Ok(())
}

#[sqlx::test(migrations = "../db/migrations")]
async fn changed_selected_provenance_and_alias_membership_require_new_ids(
    pool: sqlx::PgPool,
) -> anyhow::Result<()> {
    seed_load_member(&pool).await?;
    let (_artifacts, mut load) = resolved_test_run().await?;
    keep_declarations(&mut load, &[1]);
    let repository = ExposedDatabase::from(pool.clone());
    repository.load_declarations(&load).await?;
    let original = profile_ids(&pool).await?[&1].0;
    fresh_identity_keys(&mut load);
    load.funders[0]
        .aliases
        .push("Additional source spelling".to_owned());
    repository.load_declarations(&load).await?;
    let alias_changed = profile_ids(&pool).await?[&1].0;
    assert_ne!(alias_changed, original);

    fresh_identity_keys(&mut load);
    load.funding_entries[0].selected_observation_id =
        Some("another-selected-observation".to_owned());
    repository.load_declarations(&load).await?;
    let observation_changed = profile_ids(&pool).await?[&1].0;
    assert_ne!(observation_changed, alias_changed);

    fresh_identity_keys(&mut load);
    load.funding_entries[0].attribution_basis = Some(AttributionBasis::ParentPayer);
    load.funding_entries[0].selected_parent_declaration_id = Some(DeclarationId::new(900)?);
    repository.load_declarations(&load).await?;
    let basis_changed = profile_ids(&pool).await?[&1].0;
    assert_ne!(basis_changed, observation_changed);

    fresh_identity_keys(&mut load);
    load.funding_entries[0].selected_parent_declaration_id = Some(DeclarationId::new(901)?);
    repository.load_declarations(&load).await?;
    assert_ne!(profile_ids(&pool).await?[&1].0, basis_changed);
    Ok(())
}
