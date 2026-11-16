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
            lords::tests::{capture, fixture},
        },
        repositories::lords::{LordsCaptureStorage, LordsRepository},
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
async fn lords_primary_search_preserves_commons_phonetic_fallback(pool: PgPool) {
    let db = ExposedDatabase::from(pool);
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(3, "Lord Jahn")],
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
        serde_json::to_value(stored_register(&capture, 0)).unwrap()
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

fn stored_register(
    capture: &crate::domain::models::lords::CompleteLordsCapture,
    index: usize,
) -> crate::domain::models::lords::LordRegister {
    let mut register = capture.members()[index].clone();
    register
        .categories
        .retain(|category| !category.interests.is_empty());
    register
}

#[sqlx::test(migrations = "../db/migrations")]
async fn full_load_replaces_every_occurrence_and_preserves_omitted_members(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let first = capture(
        IngestionKey::default(),
        "2026-02-01T00:00:00Z",
        &[(2175, "Initial"), (2176, "Omitted")],
    );
    db.load_lords(&first).await.unwrap();
    let id = sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=2175")
        .fetch_one(&pool)
        .await
        .unwrap();
    let replacement = capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(2175, "Replaced")],
    );
    for _ in 0..2 {
        db.load_lords(&replacement).await.unwrap();
        assert_eq!(
            db.lord_register("2175".parse().unwrap())
                .await
                .unwrap()
                .unwrap(),
            stored_register(&replacement, 0)
        );
        assert_eq!(
            db.lord_register("2176".parse().unwrap())
                .await
                .unwrap()
                .unwrap(),
            stored_register(&first, 1)
        );
    }
    assert_eq!(
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=2175")
            .fetch_one(&pool)
            .await
            .unwrap(),
        id
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
        sqlx::query_scalar!(
            "SELECT count(*) FROM exposed.simple_declarations WHERE member_id=$1",
            id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(9)
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn failure_rolls_back_all_member_changes_and_declaration_replacements(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let first = capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(2, "Initial")],
    );
    db.load_lords(&first).await.unwrap();
    let before =
        sqlx::query_scalar!("SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM exposed.members m")
            .fetch_one(&pool)
            .await
            .unwrap();
    let declarations = sqlx::query_scalar!("SELECT jsonb_agg(to_jsonb(d) ORDER BY member_id,category_position,node_position) FROM exposed.simple_declarations d").fetch_one(&pool).await.unwrap();
    sqlx::query!("ALTER TABLE exposed.simple_declarations ADD CONSTRAINT reject_replacement CHECK (declaration_text IS DISTINCT FROM 'Reject replacement') NOT VALID").execute(&pool).await.unwrap();
    let mut page = fixture();
    let mut new = page["items"][0].clone();
    new["value"]["member"]["id"] = json!(1);
    let mut changed = new.clone();
    changed["value"]["member"]["id"] = json!(2);
    changed["value"]["member"]["nameDisplayAs"] = json!("Changed");
    changed["value"]["interestCategories"][0]["interests"][0]["interest"] =
        json!("Reject replacement");
    page["items"] = json!([new, changed]);
    page["totalResults"] = json!(2);
    page["take"] = json!(2);
    let mut builder = crate::domain::models::lords::LordsCaptureBuilder::new(
        IngestionKey::default(),
        chrono::Utc::now(),
    );
    builder
        .push_page(serde_json::to_vec(&page).unwrap())
        .unwrap();
    assert!(db.load_lords(&builder.finish().unwrap()).await.is_err());
    assert_eq!(
        sqlx::query_scalar!("SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM exposed.members m")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
    assert_eq!(sqlx::query_scalar!("SELECT jsonb_agg(to_jsonb(d) ORDER BY member_id,category_position,node_position) FROM exposed.simple_declarations d").fetch_one(&pool).await.unwrap(),declarations);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn empty_register_replaces_all_previous_occurrences(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(2175, "Initial")],
    ))
    .await
    .unwrap();
    let mut page = fixture();
    page["items"][0]["value"]["interestCategories"] = json!([]);
    let mut builder = crate::domain::models::lords::LordsCaptureBuilder::new(
        IngestionKey::default(),
        chrono::Utc::now(),
    );
    builder
        .push_page(serde_json::to_vec(&page).unwrap())
        .unwrap();
    db.load_lords(&builder.finish().unwrap()).await.unwrap();
    assert!(
        db.lord_register("2175".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .categories
            .is_empty()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.simple_declarations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn seeded_membership_kinds_match_every_domain_variant_and_chamber(pool: PgPool) {
    use crate::domain::models::membership::{Chamber, MembershipKind};
    use strum::IntoEnumIterator;
    let expected = MembershipKind::iter()
        .map(|kind| (kind.name().to_owned(), kind.chamber().name().to_owned()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let rows = sqlx::query!(
        "SELECT k.id, k.name, c.id AS chamber_id, c.name AS chamber_name
         FROM exposed.membership_kinds k JOIN exposed.chambers c ON c.id = k.chamber_id"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        rows.iter()
            .all(|row| row.id.get_version_num() == 7 && row.chamber_id.get_version_num() == 7)
    );
    let stored = rows
        .into_iter()
        .map(|row| (row.name, row.chamber_name))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(stored, expected);
    let chambers = sqlx::query!("SELECT id, name FROM exposed.chambers")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(chambers.iter().all(|row| row.id.get_version_num() == 7));
    assert_eq!(
        chambers
            .into_iter()
            .map(|row| row.name)
            .collect::<std::collections::BTreeSet<_>>(),
        [
            Chamber::Commons.name().to_owned(),
            Chamber::Lords.name().to_owned()
        ]
        .into_iter()
        .collect()
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn loading_resolves_regenerated_lookup_ids(pool: PgPool) {
    let kinds = sqlx::query!("DELETE FROM exposed.membership_kinds RETURNING id, name, chamber_id")
        .fetch_all(&pool)
        .await
        .unwrap();
    let chambers = sqlx::query!("DELETE FROM exposed.chambers RETURNING id, name")
        .fetch_all(&pool)
        .await
        .unwrap();
    for chamber in chambers {
        let id = sqlx::query_scalar!(
            "INSERT INTO exposed.chambers (name) VALUES ($1) RETURNING id",
            chamber.name
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_ne!(id, chamber.id);
        assert_eq!(id.get_version_num(), 7);
        for kind in kinds.iter().filter(|kind| kind.chamber_id == chamber.id) {
            let kind_id = sqlx::query_scalar!("INSERT INTO exposed.membership_kinds (name, chamber_id) VALUES ($1, $2) RETURNING id", kind.name, id)
                .fetch_one(&pool).await.unwrap();
            assert_ne!(kind_id, kind.id);
            assert_eq!(kind_id.get_version_num(), 7);
        }
    }
    let db = ExposedDatabase::from(pool);
    let capture = capture(
        IngestionKey::default(),
        "2026-02-01T00:00:00Z",
        &[(2175, "Regenerated")],
    );
    db.load_lords(&capture).await.unwrap();
    assert_eq!(
        db.lord_register("2175".parse().unwrap())
            .await
            .unwrap()
            .unwrap(),
        stored_register(&capture, 0)
    );
}

#[sqlx::test(
    migrations = "../db/migrations",
    fixtures(
        "../../../../../db/fixtures/add_members.sql",
        "../../../../../db/fixtures/add_declarations_and_funding_entries.sql"
    )
)]
async fn lords_profile_update_preserves_member_identity_and_financial_history(pool: PgPool) {
    use crate::domain::repositories::entity_details::EntityDetailsRepo;
    let db = ExposedDatabase::from(pool.clone());
    let id = sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let before = db.member("1".parse().unwrap()).await.unwrap().unwrap();
    let financial = sqlx::query_scalar!("SELECT jsonb_build_object('declarations',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM exposed.declarations d),'funding',(SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM exposed.funding_entries e))").fetch_one(&pool).await.unwrap();
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Lord Shared")],
    ))
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        id
    );
    let after = db.member("1".parse().unwrap()).await.unwrap().unwrap();
    assert_eq!(after.member.name, "Lord Shared");
    assert!(!after.member.is_current_commons);
    assert_eq!(after.declarations.len(), before.declarations.len());
    assert_eq!(sqlx::query_scalar!("SELECT jsonb_build_object('declarations',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM exposed.declarations d),'funding',(SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM exposed.funding_entries e))").fetch_one(&pool).await.unwrap(),financial);
}
