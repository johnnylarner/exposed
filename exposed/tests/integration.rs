use std::{collections::HashMap, ffi::OsString, fs, process::Command};

use anyhow::Context;
use clap::Parser;
use exposed::{
    domain::{
        models::{entity_ingestion::IngestionKey, entity_search::EntitySearchRequest},
        repositories::parliament_member_repository::ParliamentMemberRepo,
    },
    inbound::cli::{DataArgs, run_cli},
    outbound::ExposedDatabase,
};
use sqlx::{ConnectOptions, PgPool};
use tempfile::TempDir;

use crate::common::{
    DeclarationFunding, FundingEntry, cli_declaration_fetcher_config, cli_fetcher_config,
    cli_loader_config, read_declaration_funding, search_entities, start_app,
};

mod common;

#[sqlx::test(migrations = "../db/migrations")]
async fn search_retains_canonical_identity_and_alias_provenance(
    pool: PgPool,
) -> anyhow::Result<()> {
    let id = uuid::Uuid::from_u128(1);
    sqlx::query!("INSERT INTO exposed.funders (id, funder_name, funder_kind) VALUES ($1, 'Northstar', 'Company')", id).execute(&pool).await?;
    for alias in ["Fletchers", "Fletchers Group"] {
        sqlx::query!(
            "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2)",
            id,
            alias
        )
        .execute(&pool)
        .await?;
    }
    let app = start_app(exposed::inbound::http::config::ServerConfig {
        connection_string: pool.connect_options().to_url_lossy().into(),
    })
    .await?;
    let req = EntitySearchRequest::new_strict("Fletchers".into(), 10)?;
    let results = search_entities(&app, &req).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"], id.to_string());
    assert_eq!(results[0]["name"], "Northstar");
    assert_eq!(
        results[0]["match_source"],
        serde_json::json!({"kind": "alias", "name": "Fletchers"})
    );
    let req = EntitySearchRequest::new_strict("Northstar".into(), 10)?;
    let results = search_entities(&app, &req).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"], id.to_string());
    assert_eq!(
        results[0]["match_source"],
        serde_json::json!({"kind": "name"})
    );
    Ok(())
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
        "--config".into(),
        path.into_os_string(),
        "--ingestion-key".into(),
        ingestion_key.to_string().into(),
    ])
    .unwrap();
    run_cli(args).await.unwrap();

    let url = pool.connect_options().to_url_lossy();
    let (_file, path) = cli_loader_config(&tmp);
    // The process environment must override a URL in .env.
    fs::write(tmp.path().join(".env"), "DATABASE_URL=invalid-url\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .current_dir(tmp.path())
        .env("DATABASE_URL", url.as_str())
        .args(["data", "members", "load"])
        .arg("--config")
        .arg(path)
        .args(["--ingestion-key", &ingestion_key.to_string()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let url = pool.connect_options().to_url_lossy();
    let db = ExposedDatabase::new(url.as_str()).await;

    let params =
        EntitySearchRequest::new_with_strictness("John McDonnell".into(), 10, 0.8).unwrap();
    let res = db.get_members_by_text_search_score(&params).await.unwrap();

    let (mp, _) = res.first().unwrap();
    assert_eq!(mp.name(), "John McDonnell".to_string());

    Ok(())
}

#[sqlx::test(migrations = "../db/migrations")]
async fn declaration_fetch_preserves_member_funding(pool: PgPool) -> anyhow::Result<()> {
    let members = sqlx::query!(
        "INSERT INTO exposed.members
            (parliament_member_id, name, party_id, party_name, latest_house,
             latest_membership_from, is_current_commons)
         VALUES (4613, 'Alex Burghart', 4, 'Conservative', 1, 'Brentwood and Ongar', true),
                (5030, 'Dr Simon Opher', 15, 'Labour', 1, 'Stroud', true)
         RETURNING id, parliament_member_id"
    )
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(|member| (member.parliament_member_id, member.id))
    .collect::<HashMap<_, _>>();
    let ingestion_key = IngestionKey::default();
    let tmp = TempDir::new()?;
    let url = pool.connect_options().to_url_lossy();
    let (_file, path) = cli_declaration_fetcher_config(&tmp);
    fs::write(tmp.path().join(".env"), format!("DATABASE_URL={url}\n"))?;
    let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .current_dir(tmp.path())
        .env_remove("DATABASE_URL")
        .args(["data", "declarations", "fetch"])
        .arg("--config")
        .arg(path)
        .args(["--ingestion-key", &ingestion_key.to_string()])
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let funding = read_declaration_funding(tmp.path(), &ingestion_key)?;

    let individual_donation = funding
        .iter()
        .find(|entry| entry.declaration_id == 16901)
        .context("expected Alex Burghart's donation from David Robert Meller")?;
    assert_eq!(
        individual_donation,
        &DeclarationFunding {
            declaration_id: 16901,
            member_id: members[&4613].to_string(),
            funding_entry: FundingEntry {
                donor_name: Some("David Robert Meller".into()),
                amount: Some("2000.00".into()),
                currency: Some("GBP".into()),
                payment_type: Some("Monetary".into()),
                funder_kind: Some("Individual".into()),
                company_number: None,
            },
        }
    );

    let company_donation = funding
        .iter()
        .find(|entry| entry.declaration_id == 16863)
        .context("expected Simon Opher's donation from Labour Together")?;
    assert_eq!(
        company_donation,
        &DeclarationFunding {
            declaration_id: 16863,
            member_id: members[&5030].to_string(),
            funding_entry: FundingEntry {
                donor_name: Some("Labour Together Limited".into()),
                amount: Some("5000.00".into()),
                currency: Some("GBP".into()),
                payment_type: Some("Monetary".into()),
                funder_kind: Some("Company".into()),
                company_number: Some("09630980".into()),
            },
        }
    );
    Ok(())
}

#[test]
fn database_commands_require_environment_url() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    // An empty .env prevents discovery of configuration from a parent directory.
    fs::write(tmp.path().join(".env"), "")?;
    let config = tmp.path().join("config.yaml");
    fs::write(
        &config,
        "data_dir: ./data\nbatch_size: 100\nconnection_string: obsolete-yaml-url\n",
    )?;

    for args in [
        ["data", "members", "load"],
        ["data", "declarations", "fetch"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
            .current_dir(tmp.path())
            .env_remove("DATABASE_URL")
            .args(args)
            .arg("--config")
            .arg(&config)
            .output()?;

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("DATABASE_URL must be set"), "{stderr}");
    }
    Ok(())
}
