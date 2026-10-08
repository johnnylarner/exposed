use arrow::json::ArrayWriter;
use chrono::{NaiveDate, Utc};
use exposed::{
    domain::{
        models::{
            declaration_ingestion::{
                CapturedDeclaration, CapturedFundingEntry, DeclarationId, MemberAsId,
            },
            entity_ingestion::IngestionKey,
        },
        repositories::entity_ingestion::EntityIngestionStorage,
    },
    outbound::ExposedDataPipeline,
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};
use uuid::Uuid;

fn rows(path: &Path) -> anyhow::Result<Vec<Value>> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(fs::File::open(path)?)?.build()?;
    let mut rows = Vec::new();
    for batch in reader {
        let mut writer = ArrayWriter::new(Vec::new());
        writer.write(&batch?)?;
        writer.finish()?;
        rows.extend(serde_json::from_slice::<Vec<Value>>(&writer.into_inner())?);
    }
    Ok(rows)
}
fn run(stage: &str, config: &Path, key: &IngestionKey) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_exposed"))
        .args(["data", "declarations", stage])
        .arg(config)
        .arg("--ingestion-key")
        .arg(key.to_string())
        .env_remove("DATABASE_URL")
        .output()
        .unwrap()
}
#[tokio::test]
#[ignore = "requires built CLI, SQLx compile-time database, and EXPOSED_RESOLUTION_PYTHON with pinned Splink"]
async fn clean_then_resolve_with_real_splink_preserves_occurrences_and_refuses_overwrite()
-> anyhow::Result<()> {
    let python = std::env::var("EXPOSED_RESOLUTION_PYTHON")?;
    let temporary = tempfile::tempdir()?;
    let key = IngestionKey::default();
    let root = temporary.path().join("data");
    let storage = ExposedDataPipeline::new_with_ingestion_key(&root, key.clone())?;
    let member = MemberAsId::new(Uuid::from_u128(1), 1)?;
    let date = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
    let mut captures = Vec::new();
    for id in 1..=16 {
        let (name, kind) = match id {
            1 | 2 => ("Example Charity", "Charity"),
            3..=6 => ("Gary Lubner", "Individual"),
            7 => ("HSBC UK Bank PLC", "Company"),
            8 => ("HSBC UK (Ian Stuart, CEO)", "Company"),
            9 => ("Unite Union", "Trade Union"),
            10 => ("UNITE the Union West Midlands", "Trade Union"),
            11 => ("East Midlands Unite the Union", "Trade Union"),
            12 => ("Unite the Union Parliamentary Staff Branch", "Trade Union"),
            13 => ("Northstar Consulting trading as Fletchers", "Company"),
            14 => ("Fletchers", "Company"),
            15 => ("Carlton Club", "Unincorporated association"),
            16 => (
                "Carlton Club Political Committee",
                "Unincorporated association",
            ),
            _ => unreachable!(),
        };
        let address = match id {
            1 | 2 => Some("10 Example Road, London SW1A 1AA"),
            5 => Some("20 Other Road, London SW1A 1AA"),
            6 => Some("30 Other Road, London SW1A 1AA"),
            7 => Some("1 Centenary Square, Birmingham, B1 1HQ"),
            8 => Some("1 Centenary Square Birmingham B1 1HQ"),
            15 | 16 => Some("69 St James Street, London SW1A 1PJ"),
            _ => None,
        };
        let mut fields = vec![
            json!({"name":"DonorName","value":name}),
            json!({"name":"DonorStatus","value":kind}),
            json!({"name":"Value","value":"100"}),
        ];
        if let Some(address) = address {
            fields.push(json!({"name":"DonorPublicAddress","value":address}));
        }
        let source = json!({"id":id,"category":{"id":3,"name":"Donations"},"versions":[{"register":{"id":820,"publishedDate":"2026-09-07"},"fields":fields}]});
        captures.push(CapturedDeclaration::new(
            DeclarationId::new(id)?,
            None,
            3,
            "Donations".into(),
            820,
            date,
            None,
            vec![CapturedFundingEntry::new(
                None,
                Some(name.into()),
                None,
                Some("100".into()),
                None,
                None,
                Some(kind.into()),
                None,
                None,
            )],
            Utc::now(),
            source.to_string(),
        )?);
    }
    storage.write_raw_declarations(member, &captures).await?;
    let config = temporary.path().join("resolution.yaml");
    let worker = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resolution/worker.py");
    fs::write(
        &config,
        format!(
            "data_dir: {}\nresolution_python: {}\nresolution_worker: {}\ncandidate_budget: 100\n",
            serde_json::to_string(&root)?,
            serde_json::to_string(&python)?,
            serde_json::to_string(&worker)?
        ),
    )?;
    let source_key = key;
    let key = IngestionKey::default();
    let copied = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .args(["data", "copy-latest-raw"])
        .arg(&config)
        .arg("--ingestion-key")
        .arg(key.to_string())
        .env_remove("DATABASE_URL")
        .output()?;
    assert!(
        copied.status.success(),
        "{}",
        String::from_utf8_lossy(&copied.stderr)
    );
    assert!(
        root.join(source_key.to_string())
            .join("raw/declarations")
            .is_dir()
    );
    let clean = run("clean", &config, &key);
    assert!(
        clean.status.success(),
        "{}",
        String::from_utf8_lossy(&clean.stderr)
    );
    let cleaned = root.join(key.to_string()).join("cleaned/declarations");
    let before = fs::read(cleaned.join("funders.parquet"))?;
    let payments_before = fs::read(cleaned.join("funding_entries.parquet"))?;
    let funders = rows(&cleaned.join("funders.parquet"))?;
    let names = funders
        .iter()
        .map(|row| {
            (
                row["funder_id"].as_str().unwrap(),
                row["name_raw"].as_str().unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    fs::remove_dir_all(root.join(key.to_string()).join("raw"))?;
    let resolved = run("resolve", &config, &key);
    assert!(
        resolved.status.success(),
        "{}",
        String::from_utf8_lossy(&resolved.stderr)
    );
    let output = root.join(key.to_string()).join("resolved/declarations");
    let observations = rows(&output.join("observation_resolution.parquet"))?;
    assert_eq!(observations.len(), 16);
    assert_eq!(
        observations
            .iter()
            .map(|r| r["identity_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        7
    );
    for (name, basis, count) in [
        ("Example Charity", "statistical_link", 2),
        ("Gary Lubner", "donor_name_link", 4),
    ] {
        let members = observations
            .iter()
            .filter(|row| names[row["funder_id"].as_str().unwrap()] == name)
            .collect::<Vec<_>>();
        assert_eq!(members.len(), count, "membership count for {name}");
        assert!(members.iter().all(|row| row["identity_basis"] == basis));
        assert_eq!(
            members
                .iter()
                .map(|row| row["identity_id"].as_str().unwrap())
                .collect::<BTreeSet<_>>()
                .len(),
            1,
            "one identity for {name}"
        );
    }
    for (prefix, basis, count) in [
        ("HSBC", "extracted_name_link", 2),
        ("Unite", "trade_union_family_link", 4),
    ] {
        let members = observations
            .iter()
            .filter(|row| {
                names[row["funder_id"].as_str().unwrap()]
                    .to_ascii_lowercase()
                    .contains(&prefix.to_ascii_lowercase())
            })
            .collect::<Vec<_>>();
        assert_eq!(members.len(), count, "membership count for {prefix}");
        assert!(members.iter().all(|row| row["identity_basis"] == basis));
        assert_eq!(
            members
                .iter()
                .map(|row| row["identity_id"].as_str().unwrap())
                .collect::<BTreeSet<_>>()
                .len(),
            1,
            "one identity for {prefix}"
        );
    }
    let alias_members = observations
        .iter()
        .filter(|row| {
            let name = names[row["funder_id"].as_str().unwrap()];
            name.contains("Northstar") || name == "Fletchers"
        })
        .collect::<Vec<_>>();
    assert_eq!(alias_members.len(), 2);
    assert!(
        alias_members
            .iter()
            .all(|row| row["identity_basis"] == "extracted_name_link")
    );
    assert_eq!(
        alias_members
            .iter()
            .map(|row| row["identity_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        1
    );
    let distinct_organizations = observations
        .iter()
        .filter(|row| names[row["funder_id"].as_str().unwrap()].starts_with("Carlton Club"))
        .collect::<Vec<_>>();
    assert_eq!(distinct_organizations.len(), 2);
    assert!(
        distinct_organizations
            .iter()
            .all(|row| row["identity_basis"] == "provisional_singleton")
    );
    assert_eq!(
        distinct_organizations
            .iter()
            .map(|row| row["identity_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        2,
        "sharing an address and organization-name root must not merge distinct organizations"
    );
    let attribution = rows(&output.join("payment_attribution.parquet"))?;
    assert_eq!(attribution.len(), 16);
    assert!(
        attribution
            .iter()
            .all(|row| row["attribution_basis"] == "donor")
    );
    let pairs = rows(&output.join("pair_decisions.parquet"))?;
    assert_eq!(pairs.len(), 16);
    let negative_pair = pairs
        .iter()
        .find(|pair| {
            names[pair["left_funder_id"].as_str().unwrap()].starts_with("Carlton Club")
                && names[pair["right_funder_id"].as_str().unwrap()].starts_with("Carlton Club")
        })
        .expect("the real worker must score the shared-address organization pair");
    assert_eq!(negative_pair["disposition"], "review");
    assert_eq!(negative_pair["reason"], "insufficient_exact_evidence");
    for pair in &pairs {
        let left_name = names[pair["left_funder_id"].as_str().unwrap()];
        let right_name = names[pair["right_funder_id"].as_str().unwrap()];
        if left_name.starts_with("Carlton Club") {
            assert_ne!(left_name, right_name);
            assert_eq!(pair["disposition"], "review");
            continue;
        }
        assert_eq!(pair["disposition"], "accepted");
        if left_name == "Gary Lubner" {
            assert_eq!(left_name, right_name);
            assert_eq!(pair["reason"], "exact_donor_name");
            assert!(pair["probability"].as_f64().unwrap() < 0.999);
            assert!(pair["probability"].as_f64().unwrap() > 0.0);
        } else if left_name.contains("HSBC") || right_name.contains("HSBC") {
            assert_eq!(pair["reason"], "extracted_name_evidence");
        } else if left_name.contains("Unite") || right_name.contains("Unite") {
            assert_eq!(pair["reason"], "trade_union_family");
        } else if left_name.contains("Northstar") || right_name.contains("Northstar") {
            assert_eq!(pair["reason"], "extracted_name_evidence");
        } else {
            assert_eq!(left_name, right_name);
            assert_eq!(pair["reason"], "exact_name_full_address_threshold");
        }
    }
    let manifest = fs::read(output.join("manifest.json"))?;
    let model: Value = serde_json::from_slice(&manifest)?;
    assert_eq!(model["model"]["splink_version"], "4.0.17");
    assert_eq!(model["model"]["calibrated"], false);
    assert_eq!(model["policy_version"], "funder-resolution-v3");
    assert_eq!(fs::read(cleaned.join("funders.parquet"))?, before);
    assert_eq!(
        fs::read(cleaned.join("funding_entries.parquet"))?,
        payments_before
    );
    assert!(!run("resolve", &config, &key).status.success());
    assert_eq!(fs::read(output.join("manifest.json"))?, manifest);
    Ok(())
}
