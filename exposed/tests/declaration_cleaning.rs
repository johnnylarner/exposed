use std::{
    collections::HashSet,
    fs::{self, File},
    path::Path,
    process::{Command, Output},
};

use arrow::{
    datatypes::{DataType, TimeUnit},
    json::ArrayWriter,
};
use chrono::{DateTime, NaiveDate};
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
use parquet::arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder};
use serde_json::{Value, json};
use tempfile::TempDir;
use uuid::Uuid;

fn funding(name: Option<&str>, amount: Option<&str>) -> CapturedFundingEntry {
    CapturedFundingEntry::new(
        None,
        name.map(str::to_owned),
        None,
        amount.map(str::to_owned),
        None,
        None,
        None,
        None,
        None,
    )
}

fn declaration(
    id: u32,
    entries: Vec<CapturedFundingEntry>,
    mut top_fields: Vec<Value>,
) -> anyhow::Result<CapturedDeclaration> {
    let donors = entries
        .iter()
        .map(|entry| {
            let fields = serde_json::to_value(entry).unwrap();
            [
                ("ultimate_payer_name", "UltimatePayerName"),
                ("donor_name", "Name"),
                ("payer_name", "PayerName"),
                ("amount", "Value"),
                ("payment_type", "PaymentType"),
                ("funder_kind", "DonorStatus"),
                ("company_number", "DonorCompanyIdentifier"),
                ("is_ultimate_payer_different", "IsUltimatePayerDifferent"),
            ]
            .into_iter()
            .filter(|(key, _)| !fields[key].is_null())
            .map(|(key, name)| {
                let mut field = json!({"name": name, "value": fields[key]});
                if name == "Value" && !fields["currency"].is_null() {
                    field["typeInfo"] = json!({"currencyCode": fields["currency"]});
                }
                field
            })
            .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if !donors.is_empty() {
        top_fields.push(json!({"name": "Donors", "values": donors}));
    }
    let source = json!({
        "id": id, "parentInterestId": 900, "category": {"id": 3, "name": "Donations"},
        "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "registrationDate": "2026-09-01", "fields": top_fields}]
    });
    Ok(CapturedDeclaration::new(
        DeclarationId::new(id)?,
        Some(DeclarationId::new(900)?),
        3,
        "Donations".into(),
        820,
        NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
        NaiveDate::from_ymd_opt(2026, 9, 1),
        entries,
        DateTime::from_timestamp(1_790_985_600, 123_000_000).unwrap(),
        source.to_string(),
    )?)
}

fn run_clean(root: &Path, key: Option<&IngestionKey>) -> anyhow::Result<Output> {
    fs::write(root.join(".env"), "")?;
    fs::write(root.join("config.yaml"), "data_dir: ./data\n")?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_exposed"));
    command.current_dir(root).env_remove("DATABASE_URL").args([
        "data",
        "declarations",
        "clean",
        "--config",
        "config.yaml",
    ]);
    if let Some(key) = key {
        command.args(["--ingestion-key", &key.to_string()]);
    }
    Ok(command.output()?)
}

fn assert_success(output: &Output) -> &str {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::str::from_utf8(&output.stdout).unwrap()
}

fn read_table(path: &Path) -> anyhow::Result<Vec<Value>> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?.build()?;
    let mut json = ArrayWriter::new(Vec::new());
    for batch in reader {
        json.write(&batch?)?;
    }
    json.finish()?;
    Ok(serde_json::from_slice(&json.into_inner())?)
}

fn cleaned_path(root: &Path, key: &IngestionKey) -> std::path::PathBuf {
    root.join("data")
        .join(key.to_string())
        .join("cleaned/declarations")
}

#[tokio::test]
async fn offline_cli_preserves_occurrences_scopes_roles_and_name_features() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let key = IngestionKey::default();
    let storage =
        ExposedDataPipeline::new_with_ingestion_key(&tmp.path().join("data"), key.clone())?;
    let member = MemberAsId::new(Uuid::from_u128(1), 4613)?;
    let complete = CapturedFundingEntry::new(
        Some("Ultimate Ltd".into()),
        Some("Labour Together Limited".into()),
        Some("Intermediary".into()),
        Some("2000.00".into()),
        Some("GBP".into()),
        Some("In kind".into()),
        Some("Company".into()),
        Some("00001234".into()),
        Some(true),
    );
    let captures = [
        declaration(42, vec![complete.clone(), complete], vec![])?,
        declaration(43, vec![], vec![])?,
        declaration(44, vec![CapturedFundingEntry::default()], vec![])?,
        declaration(
            45,
            vec![],
            vec![json!({"name": "PayerName", "value": "Peters Fraser & Dunlop Limited"})],
        )?,
        declaration(
            46,
            vec![
                funding(Some("Confidential"), Some("1")),
                funding(Some("Confidential"), Some("1")),
            ],
            vec![json!({"name": "PayerName", "value": "News Corp UK & Ireland Limited"})],
        )?,
        declaration(
            47,
            vec![funding(
                Some("CSC Computer Sciences Limited (Trading as DXC Technology Ltd)"),
                None,
            )],
            vec![],
        )?,
        declaration(
            48,
            vec![CapturedFundingEntry::new(
                None,
                None,
                None,
                None,
                None,
                None,
                Some("Company".into()),
                Some("00009999".into()),
                None,
            )],
            vec![],
        )?,
    ];
    storage.write_raw_declarations(member, &captures).await?;
    storage
        .write_raw_declarations(MemberAsId::new(Uuid::from_u128(2), 5030)?, &[])
        .await?;
    let raw_path = tmp
        .path()
        .join("data")
        .join(key.to_string())
        .join("raw/declarations")
        .join(format!("{}.parquet", member.member_id()));
    let before = fs::read(&raw_path)?;
    let key = IngestionKey::default();
    fs::write(tmp.path().join(".env"), "")?;
    fs::write(tmp.path().join("config.yaml"), "data_dir: ./data\n")?;
    let copied = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .current_dir(tmp.path())
        .env_remove("DATABASE_URL")
        .args([
            "data",
            "copy-latest-raw",
            "--config",
            "config.yaml",
            "--ingestion-key",
            &key.to_string(),
        ])
        .output()?;
    assert_success(&copied);
    let copied_raw = tmp
        .path()
        .join("data")
        .join(key.to_string())
        .join("raw/declarations")
        .join(format!("{}.parquet", member.member_id()));
    assert_eq!(fs::read(&copied_raw)?, before);
    let output = run_clean(tmp.path(), Some(&key))?;
    let stdout = assert_success(&output);
    assert!(
        stdout.contains(
            "Member partitions: 2\nDeclarations: 7\nFunding entries: 7\nFunder observations: 12\n"
        ),
        "{stdout}"
    );
    assert_eq!(fs::read(raw_path)?, before);
    assert_eq!(fs::read(copied_raw)?, before);
    let path = cleaned_path(tmp.path(), &key);
    let entries = read_table(&path.join("funding_entries.parquet"))?;
    let funders = read_table(&path.join("funders.parquet"))?;
    assert_eq!(entries.len(), 7);
    assert_eq!(funders.len(), 12);
    let all_ids = funders
        .iter()
        .map(|row| row["funder_id"].as_str().unwrap())
        .collect::<HashSet<_>>();
    assert_eq!(all_ids.len(), funders.len());
    for entry in &entries {
        for field in [
            "donor_funder_id",
            "payer_funder_id",
            "ultimate_payer_funder_id",
        ] {
            if let Some(id) = entry[field].as_str() {
                assert!(all_ids.contains(id));
            }
        }
    }
    let repeated = entries
        .iter()
        .filter(|row| row["declaration_id"] == 42)
        .collect::<Vec<_>>();
    assert_eq!(repeated.len(), 2);
    assert_ne!(
        repeated[0]["funding_entry_id"],
        repeated[1]["funding_entry_id"]
    );
    assert_eq!(repeated[0]["funding_ordinal"], 0);
    assert_eq!(repeated[1]["funding_ordinal"], 1);
    assert_eq!(repeated[0]["amount"], "2000.00");
    assert_eq!(repeated[0]["company_number"], "00001234");
    let empty = entries
        .iter()
        .find(|row| row["declaration_id"] == 44)
        .unwrap();
    assert!(empty["donor_funder_id"].is_null());
    assert!(!entries.iter().any(|row| row["declaration_id"] == 43));
    for row in funders.iter().filter(|row| row["declaration_id"] == 42) {
        if row["role"] == "donor" {
            assert_eq!(row["donor_company_number"], "00001234");
            assert_eq!(row["organisation_core"], "labour together");
        } else {
            assert!(row["donor_company_number"].is_null());
        }
        let declaration = captures
            .iter()
            .find(|capture| capture.id().value() == 42)
            .unwrap();
        let source: Value = serde_json::from_str(declaration.source_json())?;
        assert!(
            source
                .pointer(row["source_pointer"].as_str().unwrap())
                .is_some()
        );
    }
    let declaration_scope = funders
        .iter()
        .filter(|row| row["source_scope"] == "declaration")
        .collect::<Vec<_>>();
    assert_eq!(declaration_scope.len(), 2);
    for row in declaration_scope {
        assert!(row["funding_entry_id"].is_null());
        assert!(row["funding_ordinal"].is_null());
        assert_eq!(row["role"], "payer");
    }
    let confidential = funders
        .iter()
        .filter(|row| row["name_status"] == "withheld")
        .collect::<Vec<_>>();
    assert_eq!(confidential.len(), 2);
    assert_ne!(confidential[0]["funder_id"], confidential[1]["funder_id"]);
    assert!(confidential[0]["name_normalized"].is_null());
    let alias = funders
        .iter()
        .find(|row| row["declaration_id"] == 47)
        .unwrap();
    assert_eq!(alias["explicit_aliases"], json!(["DXC Technology Ltd"]));
    assert_eq!(alias["organisation_core"], "csc computer sciences");
    let missing = funders
        .iter()
        .find(|row| row["declaration_id"] == 48)
        .unwrap();
    assert_eq!(missing["name_status"], "missing");
    assert_eq!(missing["donor_company_number"], "00009999");
    let schema = ParquetRecordBatchReaderBuilder::try_new(File::open(
        path.join("funding_entries.parquet"),
    )?)?;
    assert_eq!(
        schema
            .schema()
            .field_with_name("funding_ordinal")?
            .data_type(),
        &DataType::UInt32
    );
    assert_eq!(
        schema.schema().field_with_name("fetched_at")?.data_type(),
        &DataType::Timestamp(TimeUnit::Microsecond, Some("+00:00".into()))
    );
    let original_entries = fs::read(path.join("funding_entries.parquet"))?;
    let original_funders = fs::read(path.join("funders.parquet"))?;
    let repeated_output = run_clean(tmp.path(), Some(&key))?;
    assert!(!repeated_output.status.success());
    assert!(String::from_utf8_lossy(&repeated_output.stderr).contains("already exist"));
    assert_eq!(
        fs::read(path.join("funding_entries.parquet"))?,
        original_entries
    );
    assert_eq!(fs::read(path.join("funders.parquet"))?, original_funders);
    Ok(())
}

#[tokio::test]
async fn more_than_one_arrow_batch_keeps_every_row_and_repeatable_id() -> anyhow::Result<()> {
    let mut outputs = Vec::new();
    for _ in 0..2 {
        let tmp = TempDir::new()?;
        let key = IngestionKey::default();
        let storage =
            ExposedDataPipeline::new_with_ingestion_key(&tmp.path().join("data"), key.clone())?;
        let captures = (1..=2050)
            .map(|id| declaration(id, vec![funding(Some("Same donor"), Some("1"))], vec![]))
            .collect::<anyhow::Result<Vec<_>>>()?;
        storage
            .write_raw_declarations(MemberAsId::new(Uuid::from_u128(1), 4613)?, &captures)
            .await?;
        assert_success(&run_clean(tmp.path(), Some(&key))?);
        let entries = read_table(&cleaned_path(tmp.path(), &key).join("funding_entries.parquet"))?;
        let funders = read_table(&cleaned_path(tmp.path(), &key).join("funders.parquet"))?;
        assert_eq!(entries.len(), 2050);
        assert_eq!(funders.len(), 2050);
        assert_eq!(entries.last().unwrap()["declaration_id"], 2050);
        outputs.push((entries, funders));
    }
    assert_eq!(outputs[0], outputs[1]);
    Ok(())
}

#[test]
fn missing_key_or_run_fails_without_creating_storage() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let output = run_clean(tmp.path(), None)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires --ingestion-key"));
    assert!(!tmp.path().join("data").exists());
    let output = run_clean(tmp.path(), Some(&IngestionKey::default()))?;
    assert!(!output.status.success());
    assert!(!tmp.path().join("data").exists());
    Ok(())
}

#[tokio::test]
async fn rejects_malformed_sources_and_funding_projection_disagreements_before_publication()
-> anyhow::Result<()> {
    for malformed_source in [false, true] {
        let tmp = TempDir::new()?;
        let key = IngestionKey::default();
        let storage =
            ExposedDataPipeline::new_with_ingestion_key(&tmp.path().join("data"), key.clone())?;
        let member = MemberAsId::new(Uuid::from_u128(1), 4613)?;
        let valid = declaration(42, vec![funding(Some("Original"), Some("10"))], vec![])?;
        let source = if malformed_source {
            "{".to_owned()
        } else {
            valid.source_json().to_owned()
        };
        let invalid = CapturedDeclaration::new(
            valid.id(),
            valid.parent_id(),
            valid.category_id(),
            valid.category_name().to_owned(),
            valid.register_id(),
            valid.register_published_date(),
            valid.registration_date(),
            vec![funding(Some("Changed"), Some("10"))],
            valid.fetched_at(),
            source,
        )?;
        storage.write_raw_declarations(member, &[invalid]).await?;
        let output = run_clean(tmp.path(), Some(&key))?;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!cleaned_path(tmp.path(), &key).exists());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(&format!("{}.parquet", member.member_id()))
        );
    }
    Ok(())
}

#[tokio::test]
async fn empty_valid_partitions_produce_two_empty_tables_but_invalid_empty_schema_fails()
-> anyhow::Result<()> {
    for valid in [true, false] {
        let tmp = TempDir::new()?;
        let key = IngestionKey::default();
        let storage =
            ExposedDataPipeline::new_with_ingestion_key(&tmp.path().join("data"), key.clone())?;
        let member = MemberAsId::new(Uuid::from_u128(1), 4613)?;
        storage.write_raw_declarations(member, &[]).await?;
        if !valid {
            let path = tmp
                .path()
                .join("data")
                .join(key.to_string())
                .join("raw/declarations")
                .join(format!("{}.parquet", member.member_id()));
            ArrowWriter::try_new(
                File::create(path)?,
                std::sync::Arc::new(arrow::datatypes::Schema::empty()),
                None,
            )?
            .close()?;
        }
        let output = run_clean(tmp.path(), Some(&key))?;
        if valid {
            assert!(
                assert_success(&output)
                    .contains("Declarations: 0\nFunding entries: 0\nFunder observations: 0\n")
            );
            for table in ["funding_entries.parquet", "funders.parquet"] {
                assert!(read_table(&cleaned_path(tmp.path(), &key).join(table))?.is_empty());
            }
        } else {
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains("schema"));
            assert!(!cleaned_path(tmp.path(), &key).exists());
        }
    }
    Ok(())
}

#[tokio::test]
async fn reordered_raw_columns_and_abandoned_staging_do_not_change_cleaning() -> anyhow::Result<()>
{
    let tmp = TempDir::new()?;
    let key = IngestionKey::default();
    let storage =
        ExposedDataPipeline::new_with_ingestion_key(&tmp.path().join("data"), key.clone())?;
    let member = MemberAsId::new(Uuid::from_u128(1), 4613)?;
    storage
        .write_raw_declarations(
            member,
            &[declaration(
                42,
                vec![funding(Some("Example"), Some("1"))],
                vec![],
            )?],
        )
        .await?;
    let raw = tmp
        .path()
        .join("data")
        .join(key.to_string())
        .join("raw/declarations")
        .join(format!("{}.parquet", member.member_id()));
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(&raw)?)?.build()?;
    let batches = reader.collect::<Result<Vec<_>, _>>()?;
    let order = (0..batches[0].num_columns()).rev().collect::<Vec<_>>();
    let batch = batches[0].project(&order)?;
    let mut writer = ArrowWriter::try_new(File::create(raw)?, batch.schema(), None)?;
    writer.write(&batch)?;
    writer.close()?;
    let abandoned = cleaned_path(tmp.path(), &key)
        .parent()
        .unwrap()
        .join(".declarations-abandoned");
    fs::create_dir_all(&abandoned)?;
    fs::write(abandoned.join("funding_entries.parquet"), "incomplete")?;
    assert_success(&run_clean(tmp.path(), Some(&key))?);
    let path = cleaned_path(tmp.path(), &key);
    assert_eq!(read_table(&path.join("funding_entries.parquet"))?.len(), 1);
    assert_eq!(read_table(&path.join("funders.parquet"))?.len(), 1);
    assert_eq!(
        fs::read_to_string(abandoned.join("funding_entries.parquet"))?,
        "incomplete"
    );
    Ok(())
}
