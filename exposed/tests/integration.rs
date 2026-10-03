//! Integration test for the entity seacrh endpoint.
//! This test suite expects a pre-loaded database.
//!
//!

use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs::File,
    time::Duration,
};

use arrow::{
    array::{
        Array, BooleanArray, Date32Array, StringArray, TimestampMicrosecondArray, UInt32Array,
    },
    datatypes::{DataType, TimeUnit},
};

use clap::Parser;
use exposed::{
    domain::{
        models::{entity_ingestion::IngestionKey, entity_search::EntitySearchRequest},
        repositories::parliament_member_repository::ParliamentMemberRepo,
    },
    inbound::cli::{DataArgs, run_cli},
    outbound::ExposedDatabase,
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::Value;
use sqlx::{ConnectOptions, PgPool};
use tempfile::TempDir;
use tokio::time;

use crate::common::{
    cli_declaration_fetcher_config, cli_fetcher_config, cli_loader_config, search_entities,
    start_app,
};

mod common;

#[tokio::test]
async fn shows_no_hsbc_duplicates() {
    let _guard = start_app().await.unwrap();
    time::sleep(Duration::from_secs(2)).await;

    let known_duplicate = EntitySearchRequest::new_with_strictness("HSBC".into(), 10, 0.9).unwrap();
    let entities = search_entities(&known_duplicate).await.unwrap();

    let mut duplicates: HashMap<String, usize> = HashMap::new();
    for e in entities.into_iter() {
        duplicates
            .entry(e.get("name").unwrap().as_str().unwrap().into())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    assert_eq!(duplicates.get("HSBC UK Bank plc"), Some(&1));
    assert_eq!(duplicates.get("HSBC UK Bank PLC"), Some(&1));
    assert_eq!(duplicates.get("HSBC UK (Ian Stuart, CEO)"), Some(&1));
}

#[sqlx::test(migrations = "../db/migrations")]
async fn parliament_api_parses_all_sitting_members(pool: PgPool) -> sqlx::Result<()> {
    let ingestion_key = IngestionKey::default();
    let tmp = TempDir::new().unwrap();

    let (_file, path) = cli_fetcher_config(&tmp);
    let args = DataArgs::try_parse_from([
        OsString::from("exposed-data"),
        "members".into(),
        "fetch".into(),
        path.into_os_string(),
        "--ingestion-key".into(),
        ingestion_key.to_string().into(),
    ])
    .unwrap();
    run_cli(args).await.unwrap();

    let url = pool.connect_options().to_url_lossy();
    let (_file, path) = cli_loader_config(&tmp, url.to_string().as_str());
    let args = DataArgs::try_parse_from([
        OsString::from("exposed-data"),
        "members".into(),
        "load".into(),
        path.into_os_string(),
        "--ingestion-key".into(),
        ingestion_key.to_string().into(),
    ])
    .unwrap();
    run_cli(args).await.unwrap();

    let db = ExposedDatabase::from(pool);

    let params =
        EntitySearchRequest::new_with_strictness("John McDonnell".into(), 10, 0.8).unwrap();
    let res = db.get_members_by_text_search_score(&params).await.unwrap();

    let (mp, _) = res.first().unwrap();
    assert_eq!(mp.name(), "John McDonnell".to_string());

    Ok(())
}

#[sqlx::test(migrations = "../db/migrations")]
async fn declaration_fetch_captures_source_evidence_for_stored_members(
    pool: PgPool,
) -> anyhow::Result<()> {
    let ingestion_key = IngestionKey::default();
    let tmp = TempDir::new()?;
    let url = pool.connect_options().to_url_lossy();
    let (_file, path) = cli_declaration_fetcher_config(&tmp, url.as_str());
    let args = || {
        DataArgs::try_parse_from([
            OsString::from("exposed-data"),
            "declarations".into(),
            "fetch".into(),
            path.clone().into_os_string(),
            "--ingestion-key".into(),
            ingestion_key.to_string().into(),
        ])
        .unwrap()
    };
    let directory = tmp
        .path()
        .join(ingestion_key.to_string())
        .join("raw/declarations");

    let error = run_cli(args()).await.unwrap_err();
    assert!(error.to_string().contains("load members"));
    assert!(!directory.exists());

    // Alex supplies individual donors, nested donor groups, multiple versions and
    // parent/child payments. Simon supplies a company identifier with a leading zero.
    // Tony is a former MP with no records in the current Interests API.
    let members = sqlx::query!(
        "INSERT INTO exposed.members
            (parliament_member_id, name, party_id, party_name, latest_house,
             latest_membership_from, is_current_commons)
         VALUES (4613, 'Alex Burghart', 4, 'Conservative', 1, 'Brentwood and Ongar', true),
                (5030, 'Dr Simon Opher', 15, 'Labour', 1, 'Stroud', true),
                (512, 'Mr Tony Blair', 15, 'Labour', 1, 'Sedgefield', false)
         RETURNING id, parliament_member_id"
    )
    .fetch_all(&pool)
    .await?;
    let member_ids = members
        .into_iter()
        .map(|member| {
            Ok((
                u32::try_from(member.parliament_member_id)?,
                member.id.to_string(),
            ))
        })
        .collect::<anyhow::Result<HashMap<_, _>>>()?;
    let before = chrono::Utc::now().timestamp_micros();
    run_cli(args()).await?;
    let after = chrono::Utc::now().timestamp_micros();

    let manifest_bytes = std::fs::read(directory.join("manifest.json"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["ingestion_key"], ingestion_key.to_string());
    assert_eq!(manifest["member_count"], member_ids.len());
    let outputs = manifest["members"].as_array().unwrap();
    assert_eq!(outputs.len(), member_ids.len());
    let mut sources = HashMap::<u32, Value>::new();
    let mut positions = HashSet::new();
    let mut total_rows = 0;
    let mut captured_members = HashSet::new();
    let mut individual_seen = false;
    let mut company_seen = false;
    let mut donor_group_seen = false;
    let mut parent_seen = false;
    let mut nonfinancial_seen = false;
    let mut multiple_versions_seen = false;
    let mut file_snapshots = Vec::new();
    let expected_schema = [
        ("member_id", DataType::Utf8, false),
        ("parliament_member_id", DataType::UInt32, false),
        ("declaration_id", DataType::UInt32, false),
        ("parent_declaration_id", DataType::UInt32, true),
        ("category_id", DataType::UInt32, false),
        ("category_name", DataType::Utf8, false),
        ("version_index", DataType::UInt32, false),
        ("register_id", DataType::UInt32, false),
        ("register_published_date", DataType::Date32, false),
        ("registration_date", DataType::Date32, true),
        ("source_field_path", DataType::Utf8, false),
        ("ultimate_payer_name", DataType::Utf8, true),
        ("donor_name", DataType::Utf8, true),
        ("payer_name", DataType::Utf8, true),
        ("amount", DataType::Utf8, true),
        ("currency", DataType::Utf8, true),
        ("payment_type", DataType::Utf8, true),
        ("funder_kind", DataType::Utf8, true),
        ("company_number", DataType::Utf8, true),
        ("is_ultimate_payer_different", DataType::Boolean, true),
        (
            "fetched_at",
            DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        ("source_json", DataType::Utf8, false),
    ];
    for output in outputs {
        let member_id = u32::try_from(output["parliament_member_id"].as_u64().unwrap())?;
        assert!(captured_members.insert(member_id));
        assert_eq!(output["member_id"], member_ids[&member_id]);
        let file = directory.join(output["file"].as_str().unwrap());
        file_snapshots.push((file.clone(), std::fs::read(&file)?));
        let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(file)?)?;
        assert_eq!(reader.schema().fields().len(), expected_schema.len());
        for (name, data_type, nullable) in &expected_schema {
            let field = reader.schema().field_with_name(name)?;
            assert_eq!(field.data_type(), data_type);
            assert_eq!(field.is_nullable(), *nullable);
        }
        let mut member_rows = 0;
        let mut member_declarations = HashSet::new();
        // Small batches force inspection beyond the first batch.
        for batch in reader.with_batch_size(3).build()? {
            let batch = batch?;
            let text = |name: &str, row| {
                let values = batch
                    .column_by_name(name)
                    .unwrap()
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap();
                (!values.is_null(row)).then(|| values.value(row))
            };
            let integer = |name: &str, row| {
                let values = batch
                    .column_by_name(name)
                    .unwrap()
                    .as_any()
                    .downcast_ref::<UInt32Array>()
                    .unwrap();
                (!values.is_null(row)).then(|| values.value(row))
            };
            for row in 0..batch.num_rows() {
                assert_eq!(
                    text("member_id", row),
                    Some(member_ids[&member_id].as_str())
                );
                assert_eq!(integer("parliament_member_id", row), Some(member_id));
                let id = integer("declaration_id", row).unwrap();
                let version_index = integer("version_index", row).unwrap();
                let field_path = text("source_field_path", row).unwrap();
                assert!(positions.insert((id, version_index, field_path.to_string())));
                let source: Value = serde_json::from_str(text("source_json", row).unwrap())?;
                assert_eq!(source["id"], id);
                assert_eq!(source["registrant"]["memberDetail"]["id"], member_id);
                assert_eq!(
                    source["category"]["id"],
                    integer("category_id", row).unwrap()
                );
                assert_eq!(
                    source["category"]["name"].as_str(),
                    text("category_name", row)
                );
                assert_eq!(
                    source["parentInterestId"].as_u64(),
                    integer("parent_declaration_id", row).map(u64::from)
                );
                let versions = source["versions"].as_array().unwrap();
                multiple_versions_seen |= versions.len() > 1;
                let version = &versions[usize::try_from(version_index)?];
                assert_eq!(
                    version["register"]["id"],
                    integer("register_id", row).unwrap()
                );
                for (column, date) in [
                    (
                        "register_published_date",
                        &version["register"]["publishedDate"],
                    ),
                    ("registration_date", &version["registrationDate"]),
                ] {
                    let values = batch
                        .column_by_name(column)
                        .unwrap()
                        .as_any()
                        .downcast_ref::<Date32Array>()
                        .unwrap();
                    if let Some(date) = date.as_str() {
                        assert_eq!(values.value_as_date(row).unwrap().to_string(), date);
                    } else {
                        assert!(values.is_null(row));
                    }
                }
                let timestamps = batch
                    .column_by_name("fetched_at")
                    .unwrap()
                    .as_any()
                    .downcast_ref::<TimestampMicrosecondArray>()
                    .unwrap();
                assert!((before..=after).contains(&timestamps.value(row)));
                let fields = source.pointer(field_path).unwrap().as_array().unwrap();
                let value = |name: &str| {
                    fields
                        .iter()
                        .find(|field| field["name"] == name)
                        .map(|field| &field["value"])
                };
                for (column, name) in [
                    ("ultimate_payer_name", "UltimatePayerName"),
                    ("payer_name", "PayerName"),
                    ("payment_type", "PaymentType"),
                    ("funder_kind", "DonorStatus"),
                    ("company_number", "DonorCompanyIdentifier"),
                ] {
                    assert_eq!(text(column, row), value(name).and_then(Value::as_str));
                }
                let is_donor_group = field_path.contains("/values/");
                let donor = value("DonorName").and_then(Value::as_str).or_else(|| {
                    is_donor_group
                        .then(|| value("Name").and_then(Value::as_str))
                        .flatten()
                });
                assert_eq!(text("donor_name", row), donor);
                let amount = value("Value")
                    .filter(|value| !value.is_null())
                    .map(|value| {
                        value
                            .as_str()
                            .map_or_else(|| value.to_string(), str::to_string)
                    });
                assert_eq!(text("amount", row), amount.as_deref());
                let currency = fields
                    .iter()
                    .find(|field| field["name"] == "Value")
                    .and_then(|field| field["typeInfo"]["currencyCode"].as_str());
                assert_eq!(text("currency", row), currency);
                let different = batch
                    .column_by_name("is_ultimate_payer_different")
                    .unwrap()
                    .as_any()
                    .downcast_ref::<BooleanArray>()
                    .unwrap();
                assert_eq!(
                    (!different.is_null(row)).then(|| different.value(row)),
                    value("IsUltimatePayerDifferent").and_then(Value::as_bool)
                );

                if id == 16901 {
                    assert_eq!(donor, Some("David Robert Meller"));
                    assert_eq!(text("amount", row), Some("2000.00"));
                    assert_eq!(currency, Some("GBP"));
                    assert_eq!(text("payment_type", row), Some("Monetary"));
                    assert_eq!(text("funder_kind", row), Some("Individual"));
                    individual_seen = true;
                }
                if id == 16863 {
                    assert_eq!(donor, Some("Labour Together Limited"));
                    assert_eq!(text("amount", row), Some("5000.00"));
                    assert_eq!(currency, Some("GBP"));
                    assert_eq!(text("funder_kind", row), Some("Company"));
                    assert_eq!(text("company_number", row), Some("09630980"));
                    company_seen = true;
                }
                donor_group_seen |= is_donor_group && donor.is_some() && amount.is_some();
                parent_seen |= integer("parent_declaration_id", row).is_some();
                nonfinancial_seen |= amount.is_none() && donor.is_none();
                if let Some(previous) = sources.insert(id, source.clone()) {
                    assert_eq!(previous, source);
                }
                member_declarations.insert(id);
                member_rows += 1;
            }
        }
        assert_eq!(output["row_count"], member_rows);
        assert_eq!(output["declaration_count"], member_declarations.len());
        if member_id == 512 {
            assert_eq!(member_rows, 0);
        } else {
            assert!(member_rows > 0);
        }
        total_rows += member_rows;
    }
    assert!(
        individual_seen
            && company_seen
            && donor_group_seen
            && parent_seen
            && nonfinancial_seen
            && multiple_versions_seen
    );
    assert_eq!(captured_members.len(), member_ids.len());
    assert_eq!(manifest["row_count"], total_rows);
    assert_eq!(manifest["declaration_count"], sources.len());
    let mut expected_rows = 0;
    for (&id, source) in &sources {
        if let Some(parent) = source["parentInterestId"].as_u64() {
            assert!(sources.contains_key(&u32::try_from(parent)?));
        }
        for (index, version) in source["versions"].as_array().unwrap().iter().enumerate() {
            let prefix = format!("/versions/{index}/fields");
            assert!(positions.contains(&(id, u32::try_from(index)?, prefix.clone())));
            expected_rows += 1;
            for (field_index, field) in version["fields"].as_array().unwrap().iter().enumerate() {
                if field["name"] == "Donors" {
                    for group_index in 0..field["values"].as_array().unwrap().len() {
                        let path = format!("{prefix}/{field_index}/values/{group_index}");
                        assert!(positions.contains(&(id, u32::try_from(index)?, path)));
                        expected_rows += 1;
                    }
                }
            }
        }
    }
    assert_eq!(total_rows, expected_rows);
    // Compare the complete known payload, including fields outside the projection.
    let response = reqwest::Client::new()
        .get("https://interests-api.parliament.uk/api/v2/Interests/16901")
        .timeout(Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    assert_eq!(sources[&16901], serde_json::from_slice::<Value>(&response)?);

    let error = run_cli(args()).await.unwrap_err();
    assert!(error.to_string().contains("new ingestion key"));
    assert_eq!(
        std::fs::read(directory.join("manifest.json"))?,
        manifest_bytes
    );
    for (file, bytes) in file_snapshots {
        assert_eq!(std::fs::read(file)?, bytes);
    }
    Ok(())
}
