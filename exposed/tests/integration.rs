//! Integration test for the entity seacrh endpoint.
//! This test suite expects a pre-loaded database.
//!
//!

use std::{collections::HashMap, path::PathBuf, time::Duration};

use chrono::Local;
use exposed::{
    domain::{
        models::entity_search::EntitySearchRequest,
        repositories::{
            entity_ingestion_pipline::EntityIngestionStorage, parliament_api::ParliamentApi,
        },
    },
    outbound::{ExposedDataPipeline, ParliamentApiClient},
};
use tokio::time;

use crate::common::{search_entities, start_app};

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

#[tokio::test]
async fn parliament_api_parses_all_sitting_members() {
    let api = ParliamentApiClient::new(100).unwrap();

    let members = api.get_sitting_members().await.unwrap();
    assert_eq!(members.len(), 649);

    let key = Local::now().into();
    let tmp = tempfile::tempdir().unwrap();
    let buf = PathBuf::from(tmp.path());
    let fs = ExposedDataPipeline::new_with_key(&buf, key).unwrap();

    fs.write_raw_members(&members).await.unwrap();
}
