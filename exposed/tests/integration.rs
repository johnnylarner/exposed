//! Integration test for the entity seacrh endpoint.
//! This test suite expects a pre-loaded database.
//!
//!

use std::{collections::HashMap, ffi::OsString, time::Duration};

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
use tokio::time;

use crate::common::{cli_fetcher_config, cli_loader_config, search_entities, start_app};

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
