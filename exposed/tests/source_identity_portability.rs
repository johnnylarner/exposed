use std::{collections::BTreeMap, fs, path::Path, process::Command};

use arrow::json::ArrayWriter;
use chrono::{DateTime, NaiveDate};
use exposed::{
    domain::{
        models::{
            declaration_ingestion::{CapturedDeclaration, CapturedFundingEntry, DeclarationId},
            entity_ingestion::IngestionKey,
            parliament_member::MemberId,
        },
        repositories::entity_ingestion::EntityIngestionStorage,
    },
    outbound::ExposedDataPipeline,
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgPool};
use uuid::Uuid;

fn capture(id: u32, parent: Option<u32>) -> anyhow::Result<CapturedDeclaration> {
    let (fields, entries) = if parent.is_some() {
        let fields = json!([
            {"name": "DonorName", "value": "Example donor"},
            {"name": "Value", "value": "20"},
            {"name": "IsUltimatePayerDifferent", "value": false}
        ]);
        let entry = CapturedFundingEntry::new(
            None,
            Some("Example donor".into()),
            None,
            Some("20".into()),
            None,
            None,
            None,
            None,
            Some(false),
        );
        (
            json!([{"name": "Donors", "values": [fields.clone(), fields]}]),
            vec![entry.clone(), entry],
        )
    } else {
        (
            json!([
                {"name": "PayerName", "value": "Example payer"},
                {"name": "Value", "value": "20"}
            ]),
            vec![CapturedFundingEntry::new(
                None,
                None,
                Some("Example payer".into()),
                Some("20".into()),
                None,
                None,
                None,
                None,
                None,
            )],
        )
    };
    let source = json!({
        "id": id,
        "parentInterestId": parent,
        "category": {"id": 3, "name": "Donations"},
        "versions": [{
            "register": {"id": 820, "publishedDate": "2026-09-07"},
            "fields": fields
        }]
    });
    Ok(CapturedDeclaration::new(
        DeclarationId::new(id)?,
        parent.map(DeclarationId::new).transpose()?,
        3,
        "Donations".into(),
        820,
        NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
        None,
        entries,
        DateTime::from_timestamp(1_790_985_600, 123_000_000).unwrap(),
        source.to_string(),
    )?)
}

fn cli(
    stage: &str,
    root: &Path,
    key: &IngestionKey,
    database_url: Option<&str>,
) -> anyhow::Result<std::process::Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_exposed"));
    command
        .current_dir(root)
        .env_remove("DATABASE_URL")
        .args(["data", "declarations", stage])
        .arg("--config")
        .arg(root.join("config.yaml"))
        .args(["--ingestion-key", &key.to_string()]);
    if let Some(url) = database_url {
        command.env("DATABASE_URL", url);
    }
    Ok(command.output()?)
}

fn artifact_bytes(root: &Path) -> anyhow::Result<BTreeMap<std::path::PathBuf, Vec<u8>>> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(artifact_bytes(&path)?);
        } else {
            files.insert(path.clone(), fs::read(path)?);
        }
    }
    Ok(files)
}

fn parquet_rows(path: &Path) -> anyhow::Result<Vec<Value>> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(fs::File::open(path)?)?;
    assert!(reader.schema().field_with_name("member_id").is_err());
    let mut rows = Vec::new();
    for batch in reader.build()? {
        let mut writer = ArrayWriter::new(Vec::new());
        writer.write(&batch?)?;
        writer.finish()?;
        rows.extend(serde_json::from_slice::<Vec<Value>>(&writer.into_inner())?);
    }
    Ok(rows)
}

async fn insert_member(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) VALUES ($1, 4483, 'Example member', 1, 'Party', 1, 'Constituency', true)",
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn clear_import(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query!(
        "TRUNCATE exposed.declaration_load_runs, exposed.funding_entries, exposed.funder_aliases, exposed.funders, exposed.declarations, exposed.members"
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn assert_no_import(pool: &PgPool) -> anyhow::Result<()> {
    let counts = sqlx::query!(
        "SELECT (SELECT count(*) FROM exposed.declarations) AS declarations, (SELECT count(*) FROM exposed.funding_entries) AS funding_entries, (SELECT count(*) FROM exposed.funders) AS funders, (SELECT count(*) FROM exposed.funder_aliases) AS aliases, (SELECT count(*) FROM exposed.declaration_load_runs) AS runs"
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(counts.declarations, Some(0));
    assert_eq!(counts.funding_entries, Some(0));
    assert_eq!(counts.funders, Some(0));
    assert_eq!(counts.aliases, Some(0));
    assert_eq!(counts.runs, Some(0));
    Ok(())
}

async fn write_resolved_fixture(root: &Path, key: &IngestionKey) -> anyhow::Result<()> {
    let storage = ExposedDataPipeline::new_with_ingestion_key(&root.join("data"), key.clone())?;
    storage
        .write_raw_declarations(
            MemberId::new(4483)?,
            &[capture(1, None)?, capture(2, Some(1))?],
        )
        .await?;
    storage
        .write_raw_declarations(MemberId::new(5030)?, &[])
        .await?;
    fs::write(root.join(".env"), "")?;
    fs::write(
        root.join("config.yaml"),
        "data_dir: ./data\ncandidate_budget: 1000000\n",
    )?;
    for stage in ["clean", "resolve"] {
        let output = cli(stage, root, key, None)?;
        assert!(
            output.status.success(),
            "{stage}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

async fn assert_loaded_source_references(pool: &PgPool, expected_uuid: Uuid) -> anyhow::Result<()> {
    let loaded = sqlx::query!(
        "SELECT d.source_declaration_id, d.member_id, m.parliament_member_id FROM exposed.declarations d JOIN exposed.members m ON m.id = d.member_id ORDER BY d.source_declaration_id"
    )
    .fetch_all(pool)
    .await?;
    assert_eq!(loaded.len(), 2);
    for row in &loaded {
        assert_eq!(row.member_id, expected_uuid);
        assert_eq!(row.parliament_member_id, 4483);
    }
    assert_eq!(loaded[0].source_declaration_id, 1);
    assert_eq!(loaded[1].source_declaration_id, 2);
    let funding = sqlx::query!(
        "SELECT source_funding_entry_id, selected_parent_declaration_id, attribution_basis, selected_observation_id FROM exposed.funding_entries ORDER BY source_funding_entry_id"
    )
    .fetch_all(pool)
    .await?;
    assert_eq!(
        funding
            .iter()
            .map(|row| row.source_funding_entry_id.as_deref())
            .collect::<Vec<_>>(),
        vec![
            Some("4483/1/820/funding/0"),
            Some("4483/2/820/funding/0"),
            Some("4483/2/820/funding/1")
        ]
    );
    for child in &funding[1..] {
        assert_eq!(child.selected_parent_declaration_id, Some(1));
        assert_eq!(child.attribution_basis.as_deref(), Some("parent_payer"));
        assert_eq!(
            child.selected_observation_id.as_deref(),
            Some("4483/1/820/funding/0/payer")
        );
    }
    Ok(())
}

fn checked_source_artifacts(
    run: &Path,
    database_members: &[Uuid],
) -> anyhow::Result<BTreeMap<std::path::PathBuf, Vec<u8>>> {
    assert!(run.join("raw/declarations/4483.parquet").is_file());
    assert!(run.join("raw/declarations/5030.parquet").is_file());
    assert!(parquet_rows(&run.join("raw/declarations/5030.parquet"))?.is_empty());
    let unchanged = artifact_bytes(run)?;
    for path in unchanged.keys().filter(|path| {
        path.extension()
            .is_some_and(|extension| extension == "parquet")
    }) {
        for row in parquet_rows(path)? {
            let text = row.to_string();
            for member in database_members {
                assert!(!text.contains(&member.to_string()));
            }
        }
    }
    let payments = parquet_rows(&run.join("cleaned/declarations/funding_entries.parquet"))?;
    assert_eq!(payments.len(), 3);
    assert_eq!(payments[0]["parliament_member_id"], 4483);
    assert_eq!(payments[0]["funding_entry_id"], "4483/1/820/funding/0");
    assert_eq!(payments[1]["funding_entry_id"], "4483/2/820/funding/0");
    assert_eq!(payments[2]["funding_entry_id"], "4483/2/820/funding/1");
    Ok(unchanged)
}

#[sqlx::test(migrations = "../db/migrations")]
async fn unchanged_source_artifacts_load_after_member_uuid_rebuild(
    pool: PgPool,
) -> anyhow::Result<()> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    let key = IngestionKey::default();
    let old_uuid = Uuid::from_u128(1);
    let new_uuid = Uuid::from_u128(2);
    insert_member(&pool, old_uuid).await?;
    write_resolved_fixture(root, &key).await?;
    let run = root.join("data").join(key.to_string());
    let unchanged = checked_source_artifacts(&run, &[old_uuid, new_uuid])?;
    let url = pool.connect_options().to_url_lossy();
    for expected_uuid in [old_uuid, new_uuid] {
        let output = cli("load", root, &key, Some(url.as_str()))?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_loaded_source_references(&pool, expected_uuid).await?;
        let retry = cli("load", root, &key, Some(url.as_str()))?;
        assert!(
            retry.status.success(),
            "{}",
            String::from_utf8_lossy(&retry.stderr)
        );
        assert!(String::from_utf8_lossy(&retry.stdout).contains("Run already loaded"));
        let counts = sqlx::query!(
            "SELECT (SELECT count(*) FROM exposed.declarations) AS declarations, (SELECT count(*) FROM exposed.funding_entries) AS funding_entries, (SELECT count(*) FROM exposed.declaration_load_runs) AS runs"
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(counts.declarations, Some(2));
        assert_eq!(counts.funding_entries, Some(3));
        assert_eq!(counts.runs, Some(1));
        assert_eq!(artifact_bytes(&run)?, unchanged);
        clear_import(&pool).await?;
        if expected_uuid == old_uuid {
            insert_member(&pool, new_uuid).await?;
        }
    }
    let missing = cli("load", root, &key, Some(url.as_str()))?;
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr)
            .contains("member 4483 must be loaded before declarations")
    );
    assert_no_import(&pool).await?;
    insert_member(&pool, new_uuid).await?;
    sqlx::query!(
        "CREATE FUNCTION exposed.reject_test_declaration() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.source_declaration_id = 2 THEN RAISE EXCEPTION 'injected declaration failure'; END IF; RETURN NEW; END $$"
    )
    .execute(&pool)
    .await?;
    sqlx::query!(
        "CREATE TRIGGER reject_test_declaration BEFORE INSERT ON exposed.declarations FOR EACH ROW EXECUTE FUNCTION exposed.reject_test_declaration()"
    )
    .execute(&pool)
    .await?;
    let failed = cli("load", root, &key, Some(url.as_str()))?;
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("injected declaration failure"));
    assert_no_import(&pool).await?;
    assert_eq!(artifact_bytes(&run)?, unchanged);
    Ok(())
}
