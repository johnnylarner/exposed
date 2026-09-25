use super::date;
use crate::imports::{
    adapters::{
        parliament::{Parliament, Request, Response, Transport},
        postgres::PostgresStore,
    },
    core::{
        ImportError, Result,
        coordinator::{Clock, ImportKind, run_import},
        ports::ImportStore,
        refresh::{refresh_declarations, refresh_members},
    },
};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use std::{collections::BTreeMap, sync::Mutex};

pub(super) fn profile(id: i32) -> Value {
    json!({"value":{"id":id,"nameDisplayAs":format!("Member {id}"),"latestParty":{"id":1,"name":"Party"},"latestHouseMembership":{"house":1,"membershipFrom":"Seat"}}})
}
pub(super) fn declaration(id: i32, member: i32, amount: &str) -> Value {
    json!({"id":id,"category":{"id":1,"name":"Employment","type":"Commons"},"registrant":{"type":"Member","memberDetail":{"id":member}},"versions":[{"register":{"publishedDate":"2026-09-01"},"fields":[{"name":"DonorName","value":" Example Limited "},{"name":"Value","type":"Decimal","value":amount,"typeInfo":{"currencyCode":"GBP"}}]}]})
}

pub(super) struct Fixture {
    pub members: Vec<i32>,
    pub declarations: BTreeMap<i32, Vec<Value>>,
    pub fail_at: Option<(i32, usize)>,
    pub parents: BTreeMap<i32, Value>,
    pub requests: Mutex<Vec<Request>>,
    pub visible_before_next_page: Option<PgPool>,
}
impl Fixture {
    pub fn new(members: Vec<i32>) -> Self {
        Self {
            members,
            declarations: BTreeMap::new(),
            fail_at: None,
            parents: BTreeMap::new(),
            requests: Mutex::new(Vec::new()),
            visible_before_next_page: None,
        }
    }
}
impl Transport for Fixture {
    async fn fetch(&self, request: Request) -> Result<Response> {
        self.requests.lock().unwrap().push(request.clone());
        let payload = match request {
            Request::CurrentMembers { offset } | Request::MemberCandidates { offset, .. } => {
                if matches!(request, Request::MemberCandidates { .. }) && offset > 0 {
                    if let Some(pool) = &self.visible_before_next_page {
                        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM exposed.members").fetch_one(pool).await.unwrap();
                        assert_eq!(count, offset as i64, "previous batch must be visible before fetching the next page");
                    }
                }
                json!({"items":self.members.iter().skip(offset).take(1).map(|id| profile(*id)).collect::<Vec<_>>(),"skip":offset,"totalResults":self.members.len()})
            }
            Request::MemberHistories { ids } => json!(ids.iter().map(|id| json!({"value":{"id":id,"houseMembershipHistory":[{"house":1,"membershipStartDate":"1987-06-11"}]}})).collect::<Vec<_>>()),
            Request::Declarations { member, offset } => {
                if self.fail_at == Some((member, offset)) { return Err(ImportError::Source("Late page failed".into())); }
                let items = self.declarations.get(&member).cloned().unwrap_or_default();
                json!({"items":items.iter().skip(offset).take(2).collect::<Vec<_>>(),"skip":offset,"totalResults":items.len()})
            }
            Request::Parent { parent, .. } => json!({"items":self.parents.get(&parent).into_iter().collect::<Vec<_>>(),"skip":0,"totalResults":usize::from(self.parents.contains_key(&parent))}),
        };
        Ok(Response {
            payload,
            fetched_at: Utc::now(),
            context: "fixture".into(),
        })
    }
}

async fn seed(store: &PostgresStore, members: Vec<i32>) {
    refresh_members(
        date("2024-07-04"),
        date("2026-09-15"),
        &Parliament(Fixture::new(members)),
        store,
    )
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../db/migrations")]
async fn next_member_page_is_not_fetched_until_previous_batch_is_committed(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    let mut source = Fixture::new(vec![1, 2, 3]);
    source.visible_before_next_page = Some(pool);
    let result = refresh_members(
        date("2024-07-04"),
        date("2026-09-15"),
        &Parliament(source),
        &store,
    )
    .await
    .unwrap();
    assert_eq!(result.members, 3);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn late_source_failure_retains_completed_member_but_no_partial_current_member(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    seed(&store, vec![1, 2]).await;
    let mut source = Fixture::new(vec![1, 2]);
    source
        .declarations
        .insert(1, vec![declaration(101, 1, "10")]);
    source.declarations.insert(
        2,
        vec![
            declaration(102, 2, "20"),
            declaration(103, 2, "30"),
            declaration(104, 2, "40"),
        ],
    );
    source.fail_at = Some((2, 2));
    assert!(
        refresh_declarations(
            date("2024-07-04"),
            date("2026-09-15"),
            &Parliament(source),
            &store
        )
        .await
        .is_err()
    );
    let ids: Vec<i32> =
        sqlx::query_scalar("SELECT source_declaration_id FROM exposed.declarations")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(ids, vec![101]);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn rejected_update_retains_funding_and_accepted_siblings_commit(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    seed(&store, vec![1]).await;
    let mut source = Fixture::new(vec![1]);
    source.declarations.insert(
        1,
        vec![declaration(101, 1, "10"), declaration(102, 1, "20")],
    );
    refresh_declarations(
        date("2024-07-04"),
        date("2026-09-15"),
        &Parliament(source),
        &store,
    )
    .await
    .unwrap();
    let previous: uuid::Uuid = sqlx::query_scalar(
        "SELECT id FROM exposed.funding_entries WHERE source_declaration_id=101",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut source = Fixture::new(vec![1, 9]);
    source.declarations.insert(
        1,
        vec![declaration(101, 1, "bad"), declaration(102, 1, "30")],
    );
    let result = refresh_declarations(
        date("2024-07-04"),
        date("2026-09-15"),
        &Parliament(source),
        &store,
    )
    .await
    .unwrap();
    assert_eq!(result.rejected, 1);
    assert_eq!(result.new_members, vec![9]);
    let rows = sqlx::query(
        "SELECT id, amount::TEXT FROM exposed.funding_entries ORDER BY source_declaration_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows[0].get::<uuid::Uuid, _>("id"), previous);
    assert_eq!(rows[0].get::<String, _>("amount"), "10");
    assert_eq!(rows[1].get::<String, _>("amount"), "30");
}

struct FixedClock(chrono::DateTime<Utc>);
impl Clock for FixedClock {
    fn now(&self) -> chrono::DateTime<Utc> {
        self.0
    }
}

#[sqlx::test(migrations = "../db/migrations")]
async fn failed_refresh_does_not_advance_freshness_and_restart_retains_retry_time(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    seed(&store, vec![1]).await;
    let now = date("2026-09-15")
        .and_time(chrono::NaiveTime::MIN)
        .and_utc();
    let source = Parliament(Fixture::new(vec![1]));
    run_import(
        ImportKind::Declarations,
        date("2024-07-04"),
        date("2026-09-15"),
        &source,
        &store,
        &FixedClock(now),
        None,
        true,
    )
    .await
    .unwrap();
    let mut source = Fixture::new(vec![1]);
    source.fail_at = Some((1, 0));
    let later = now + Duration::days(1);
    assert!(
        run_import(
            ImportKind::Declarations,
            date("2024-07-04"),
            date("2026-09-16"),
            &Parliament(source),
            &store,
            &FixedClock(later),
            None,
            true
        )
        .await
        .is_err()
    );
    let restarted = PostgresStore::new(pool);
    let state = restarted.refresh_state(date("2024-07-04")).await.unwrap();
    assert_eq!(state.last_completed, Some(now));
    assert_eq!(state.next_attempt, Some(later + Duration::minutes(5)));
    assert!(!state.due(later));
    assert!(state.due(later + Duration::minutes(5)));
}

#[sqlx::test(migrations = "../db/migrations")]
async fn single_import_guard_survives_multiple_application_instances(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    let second = PostgresStore::new(pool);
    let lease = store.acquire().await.unwrap();
    assert!(matches!(second.acquire().await, Err(ImportError::Busy)));
    drop(lease);
    // Closing the lease releases a server-side session lock asynchronously.
    for _ in 0..50 {
        if let Ok(lease) = second.acquire().await {
            drop(lease);
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("Import lock was not released");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn complete_with_rejections_advances_check_time_and_parents_are_resolved_only_when_needed(
    pool: PgPool,
) {
    let store = PostgresStore::new(pool.clone());
    seed(&store, vec![1]).await;
    let mut child = declaration(101, 1, "12.50");
    child["parentInterestId"] = json!(500);
    child["versions"][0]["fields"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let parent = declaration(500, 1, "15");
    let mut source = Fixture::new(vec![1]);
    source
        .declarations
        .insert(1, vec![child, declaration(102, 1, "bad")]);
    source.parents.insert(500, parent);
    let now = Utc::now();
    let summary = run_import(
        ImportKind::Declarations,
        date("2024-07-04"),
        date("2026-09-15"),
        &Parliament(source),
        &store,
        &FixedClock(now),
        None,
        false,
    )
    .await
    .unwrap();
    assert_eq!(summary["rejected"], 1);
    let name: String = sqlx::query_scalar("SELECT f.funder_name FROM exposed.funders f JOIN exposed.funding_entries e ON e.funder_id=f.id WHERE e.source_declaration_id=101").fetch_one(&pool).await.unwrap();
    assert_eq!(name, "example ltd");
    let state = store.refresh_state(date("2024-07-04")).await.unwrap();
    assert_eq!(state.last_completed, Some(now));
    assert_eq!(state.outcome.as_deref(), Some("warnings"));
}

#[sqlx::test(migrations = "../db/migrations")]
async fn conflicting_unknown_source_fields_fail_before_any_member_publication(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    seed(&store, vec![1]).await;
    for (first, second) in [
        (json!(false), json!(true)),
        (json!(-0.0), json!(0.0)),
        (
            serde_json::from_str("1000000000000000000000000000001").unwrap(),
            serde_json::from_str("1000000000000000000000000000002").unwrap(),
        ),
    ] {
        let mut original = declaration(101, 1, "10");
        let mut conflicting = original.clone();
        original["unknownFutureField"] = first;
        conflicting["unknownFutureField"] = second;
        let mut source = Fixture::new(vec![1]);
        source.declarations.insert(1, vec![original, conflicting]);
        let result = refresh_declarations(
            date("2024-07-04"),
            date("2026-09-15"),
            &Parliament(source),
            &store,
        )
        .await;
        assert!(matches!(result, Err(ImportError::Invalid(_))));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM exposed.declarations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn absent_cohort_fails_before_any_source_request(pool: PgPool) {
    let source = Parliament(Fixture::new(vec![]));
    assert!(
        refresh_declarations(
            date("2024-07-04"),
            date("2026-09-15"),
            &source,
            &PostgresStore::new(pool)
        )
        .await
        .is_err()
    );
    assert!(source.0.requests.lock().unwrap().is_empty());
}
