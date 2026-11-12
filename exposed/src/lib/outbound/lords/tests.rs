use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

use super::*;
use crate::{
    domain::{
        models::{
            entity_ingestion::IngestionKey,
            entity_search::{Entity, EntitySearchRequest},
            lords::{
                LordsLoadOutcome,
                tests::{capture, fixture},
            },
        },
        repositories::{
            entity_details::EntityDetailsRepo,
            lords::{LordsCaptureStorage, LordsRepository},
        },
        services::{
            entity_search::{EntitySearchService, Service as SearchService},
            lords::LordsFetcher,
        },
    },
    inbound::http::{routes::routes, state::AppState},
    outbound::ExposedDatabase,
};

struct Source(Mutex<VecDeque<Result<Vec<u8>, LordsError>>>);

impl LordsSource for Source {
    fn page(&self, page: u32) -> impl Future<Output = Result<Vec<u8>, LordsError>> {
        assert!(page >= 1);
        std::future::ready(
            self.0
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected extra request"),
        )
    }
}

#[tokio::test]
async fn failed_or_repeated_page_never_publishes_a_partial_capture() {
    let mut first = fixture();
    first["totalResults"] = json!(2);
    first["take"] = json!(1);
    let bytes = serde_json::to_vec(&first).unwrap();
    for second in [
        Err(LordsError::Source("interrupted second page".into())),
        Ok(bytes.clone()),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let key = IngestionKey::default();
        let storage = LordsCaptureFiles::new(directory.path(), key.clone()).unwrap();
        let destination = storage.path().to_owned();
        let source = Source(Mutex::new(VecDeque::from([Ok(bytes.clone()), second])));
        assert!(LordsFetcher::new(source, storage).fetch(key).await.is_err());
        assert!(!destination.exists());
    }
}

#[tokio::test]
async fn complete_artifact_replays_exact_bytes_and_rejects_tampering_or_republication() {
    let directory = tempfile::tempdir().unwrap();
    let key = IngestionKey::default();
    let storage = LordsCaptureFiles::new(directory.path(), key.clone()).unwrap();
    let capture = capture(key, "2026-11-12T00:00:00Z", &[(2175, "Lord Example")]);
    storage.publish(&capture).await.unwrap();
    let replay = storage.read().await.unwrap();
    assert_eq!(replay.pages(), capture.pages());
    assert_eq!(replay.members()[0].raw(), capture.members()[0].raw());
    assert!(storage.publish(&capture).await.is_err());
    let page = storage.path().join("page-1.json");
    let mut tampered = capture.pages()[0].clone();
    tampered.push(b' ');
    std::fs::write(&page, tampered).unwrap();
    assert!(
        storage
            .read()
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("digest")
    );
}

#[sqlx::test(
    migrations = "../db/migrations",
    fixtures("../../../../../db/fixtures/add_members.sql")
)]
async fn load_is_idempotent_monotonic_and_isolated_from_commons(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let key = IngestionKey::default();
    let first = capture(
        key.clone(),
        "2026-11-10T00:00:00Z",
        &[(1, "Lord First"), (2, "Lord Omitted")],
    );
    assert_eq!(
        db.load_lords(&first).await.unwrap(),
        LordsLoadOutcome::Loaded
    );
    assert_eq!(
        db.load_lords(&first).await.unwrap(),
        LordsLoadOutcome::AlreadyLoaded
    );
    let conflict = capture(key, "2026-11-10T00:00:00Z", &[(1, "Lord Changed")]);
    assert!(matches!(
        db.load_lords(&conflict).await,
        Err(LordsError::Conflict)
    ));
    let newer = capture(
        IngestionKey::default(),
        "2026-11-12T00:00:00Z",
        &[(1, "Lord Newer")],
    );
    db.load_lords(&newer).await.unwrap();
    let older = capture(
        IngestionKey::default(),
        "2026-11-11T00:00:00Z",
        &[(1, "Lord Older")],
    );
    db.load_lords(&older).await.unwrap();
    assert_eq!(
        db.lord_register("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "Lord Newer"
    );
    assert_eq!(
        db.lord_register("2".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "Lord Omitted"
    );
    assert_eq!(
        db.member("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "John McDonnell"
    );
    let row = sqlx::query!(
        "SELECT source_document AS \"source_document!\" FROM exposed.member_profiles p JOIN exposed.members m ON m.id = p.member_id AND p.chamber = 'lords' WHERE parliament_member_id = 1"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.source_document, *newer.members()[0].raw());
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_imports")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(3)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.declarations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.funding_entries")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.funders")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn failed_member_write_rolls_back_every_document_and_the_ledger(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let initial = capture(
        IngestionKey::default(),
        "2026-11-10T00:00:00Z",
        &[(1, "Lord Initial")],
    );
    db.load_lords(&initial).await.unwrap();
    sqlx::query!(
        "ALTER TABLE exposed.member_profiles ADD CONSTRAINT reject_test_name CHECK (name <> 'Reject')"
    )
    .execute(&pool)
    .await
    .unwrap();
    let bad = capture(
        IngestionKey::default(),
        "2026-11-12T00:00:00Z",
        &[(1, "Lord Changed"), (2, "Reject")],
    );
    assert!(db.load_lords(&bad).await.is_err());
    assert_eq!(
        db.lord_register("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "Lord Initial"
    );
    assert!(
        db.lord_register("2".parse().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_imports")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn concurrent_imports_resolve_against_the_locked_row_and_break_timestamp_ties(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.load_lords(&capture(
        IngestionKey::default(),
        "2025-01-01T00:00:00Z",
        &[(1, "Lord Initial")],
    ))
    .await
    .unwrap();
    for (round, older_date, newer_date, newest_first) in [
        (0, "2026-01-01T00:00:00Z", "2026-02-01T00:00:00Z", false),
        (1, "2027-01-01T00:00:00Z", "2027-02-01T00:00:00Z", true),
        (2, "2028-01-01T00:00:00Z", "2028-01-01T00:00:00Z", false),
        (3, "2029-01-01T00:00:00Z", "2029-01-01T00:00:00Z", true),
    ] {
        let mut lock = pool.begin().await.unwrap();
        sqlx::query!("SELECT parliament_member_id FROM exposed.members WHERE parliament_member_id = 1 FOR UPDATE").fetch_one(&mut *lock).await.unwrap();
        let older = capture(
            IngestionKey::new(uuid::Uuid::from_u128(10 + round * 2)),
            older_date,
            &[(1, "Lord Older")],
        );
        let newer = capture(
            IngestionKey::new(uuid::Uuid::from_u128(11 + round * 2)),
            newer_date,
            &[(1, "Lord Newer")],
        );
        let expected = newer.members()[0].register().clone();
        let expected_evidence = newer.members()[0].raw().clone();
        let expected_key = newer.key().uuid();
        let (first, second) = if newest_first {
            (newer, older)
        } else {
            (older, newer)
        };
        let left = db.clone();
        let right = db.clone();
        let first = tokio::spawn(async move { left.load_lords(&first).await });
        wait_for_identity_locks(&pool, 1).await;
        let second = tokio::spawn(async move { right.load_lords(&second).await });
        wait_for_identity_locks(&pool, 2).await;
        lock.commit().await.unwrap();
        assert_eq!(first.await.unwrap().unwrap(), LordsLoadOutcome::Loaded);
        assert_eq!(second.await.unwrap().unwrap(), LordsLoadOutcome::Loaded);
        assert_eq!(
            db.lord_register("1".parse().unwrap())
                .await
                .unwrap()
                .unwrap(),
            expected
        );
        let stored = sqlx::query!(
            "SELECT source_document AS \"source_document!\", ingestion_key AS \"ingestion_key!\" FROM exposed.member_profiles WHERE chamber = 'lords'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored.source_document, expected_evidence);
        assert_eq!(stored.ingestion_key, expected_key);
    }
    let capture = capture(
        IngestionKey::default(),
        "2030-01-01T00:00:00Z",
        &[(1, "Lord Retry")],
    );
    let (left, right) = tokio::join!(db.load_lords(&capture), db.load_lords(&capture));
    let outcomes = [left.unwrap(), right.unwrap()];
    assert!(outcomes.contains(&LordsLoadOutcome::Loaded));
    assert!(outcomes.contains(&LordsLoadOutcome::AlreadyLoaded));
}

#[sqlx::test(
    migrations = "../db/migrations",
    fixtures("../../../../../db/fixtures/add_members.sql")
)]
async fn lords_primary_search_preserves_commons_phonetic_fallback(pool: PgPool) {
    let db = ExposedDatabase::from(pool);
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Lord Jahn")],
    ))
    .await
    .unwrap();
    let results = SearchService::new(db.clone(), db)
        .search_entities(&EntitySearchRequest::new_with_strictness("jahn".into(), 10, 0.8).unwrap())
        .await
        .unwrap();
    assert!(matches!(&results[0], Entity::Lord(profile) if profile.name == "Lord Jahn"));
    assert_eq!(
        results.iter().skip(1).map(Entity::name).collect::<Vec<_>>(),
        ["John McDonnell", "John Humphries"]
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn public_lords_response_and_search_keep_the_frozen_contract(pool: PgPool) {
    let db = ExposedDatabase::from(pool);
    let capture = capture(
        IngestionKey::default(),
        "2026-11-12T00:00:00Z",
        &[(2175, "Lord Example")],
    );
    db.load_lords(&capture).await.unwrap();
    let app = routes().with_state(AppState {
        updates_service: Arc::new(crate::domain::services::updates::Service::new(db.clone())),
        rankings_service: Arc::new(crate::domain::services::rankings::Service::new(db.clone())),
        entity_search_service: Arc::new(SearchService::new(db.clone(), db.clone())),
        entity_details_service: Arc::new(crate::domain::services::entity_details::Service::new(db)),
    });
    for (url, status) in [
        ("/lords/0", StatusCode::UNPROCESSABLE_ENTITY),
        ("/lords/no", StatusCode::UNPROCESSABLE_ENTITY),
        ("/lords/2147483648", StatusCode::UNPROCESSABLE_ENTITY),
        ("/lords/2176", StatusCode::NOT_FOUND),
        ("/lords/2175/history", StatusCode::NOT_FOUND),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(url).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{url}");
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/lords/2175")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let public: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        public,
        serde_json::to_value(capture.members()[0].register()).unwrap()
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?term=Example&max_entries=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let public: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        public["entities"][0],
        json!({"id":"2175", "name":"Lord Example", "kind":"Lords", "funder_kind":null, "match_source":{"kind":"name"}})
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn relational_occurrences_preserve_source_values_without_json_reads(pool: PgPool) {
    use crate::domain::models::lords::LordsCaptureBuilder;
    let mut source = fixture();
    let value = &mut source["items"][0]["value"];
    value["member"]["latestHouseMembership"]["membershipEndDate"] =
        json!("2024-03-02T01:02:03.123400+03:30");
    let node = json!({
        "id": u32::MAX, "interest": "  ", "isCorrection": true,
        "createdWhen": "2020-01-01T00:00:00",
        "lastAmendedWhen": "2020-01-02T00:00:00.1200Z",
        "deletedWhen": "2020-01-03T00:00:00-04:00",
        "childInterests": [{"id": u32::MAX, "interest": null,
            "isCorrection": false, "deletedWhen": "2020-01-01T00:00:00Z"}]
    });
    value["interestCategories"] = json!([
        {"id":u32::MAX,"name":null,"sortOrder":-10,"interests":[node.clone(),node]},
        {"id":u32::MAX,"name":"","sortOrder":-10,"interests":[]}
    ]);
    let mut builder = LordsCaptureBuilder::new(
        IngestionKey::default(),
        "2026-11-12T00:00:00Z".parse().unwrap(),
    );
    builder
        .push_page(serde_json::to_vec(&source).unwrap())
        .unwrap();
    let capture = builder.finish().unwrap();
    let expected = capture.members()[0].register();
    let db = ExposedDatabase::from(pool.clone());
    db.load_lords(&capture).await.unwrap();
    assert_eq!(
        &db.lord_register(expected.member.id).await.unwrap().unwrap(),
        expected
    );
    sqlx::query!(
        "UPDATE exposed.member_profiles SET source_document = '{}'::jsonb WHERE chamber = 'lords'"
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        &db.lord_register(expected.member.id).await.unwrap().unwrap(),
        expected
    );
    let search =
        EntitySearchRequest::new_with_strictness(expected.member.name.clone(), 10, 1.0).unwrap();
    assert_eq!(
        db.search_lords(&search).await.unwrap()[0].0,
        expected.member
    );
}

mod relational;

async fn wait_for_identity_locks(pool: &PgPool, count: i64) {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting = sqlx::query_scalar!("SELECT count(*) FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock' AND query LIKE 'INSERT INTO exposed.members%'").fetch_one(pool).await.unwrap().unwrap();
            if waiting == count { break; }
            tokio::task::yield_now().await;
        }
    }).await.expect("imports reached the identity lock barrier");
}
