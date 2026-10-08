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
    for id in 1..=6 {
        let name = if id <= 2 {
            "Example Charity"
        } else {
            "Gary Lubner"
        };
        let kind = if id <= 2 { "Charity" } else { "Individual" };
        let address = match id {
            1 | 2 => Some("10 Example Road, London SW1A 1AA"),
            5 => Some("20 Other Road, London SW1A 1AA"),
            6 => Some("30 Other Road, London SW1A 1AA"),
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
    assert_eq!(observations.len(), 6);
    assert_eq!(
        observations
            .iter()
            .map(|r| r["identity_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        2
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
    let attribution = rows(&output.join("payment_attribution.parquet"))?;
    assert_eq!(attribution.len(), 6);
    assert!(
        attribution
            .iter()
            .all(|row| row["attribution_basis"] == "donor")
    );
    let pairs = rows(&output.join("pair_decisions.parquet"))?;
    assert_eq!(pairs.len(), 7);
    assert!(pairs.iter().all(|pair| pair["disposition"] == "accepted"));
    for pair in &pairs {
        let left_name = names[pair["left_funder_id"].as_str().unwrap()];
        let right_name = names[pair["right_funder_id"].as_str().unwrap()];
        assert_eq!(left_name, right_name);
        if left_name == "Gary Lubner" {
            assert_eq!(pair["reason"], "exact_donor_name");
            assert!(pair["probability"].as_f64().unwrap() < 0.999);
            assert!(pair["probability"].as_f64().unwrap() > 0.0);
        } else {
            assert_eq!(pair["reason"], "exact_name_full_address_threshold");
        }
    }
    let manifest = fs::read(output.join("manifest.json"))?;
    let model: Value = serde_json::from_slice(&manifest)?;
    assert_eq!(model["model"]["splink_version"], "4.0.17");
    assert_eq!(model["model"]["calibrated"], false);
    assert_eq!(model["policy_version"], "funder-resolution-v2");
    assert_eq!(fs::read(cleaned.join("funders.parquet"))?, before);
    assert_eq!(
        fs::read(cleaned.join("funding_entries.parquet"))?,
        payments_before
    );
    assert!(!run("resolve", &config, &key).status.success());
    assert_eq!(fs::read(output.join("manifest.json"))?, manifest);
    Ok(())
}
