//! Integration test for the entity seacrh endpoint.
//! This test suite expects a pre-loaded database.
//!
//!

use std::{collections::HashMap, time::Duration};

use exposed::domain::models::entity_search::EntitySearchRequest;
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
