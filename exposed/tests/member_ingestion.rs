//! One executable workflow against real HTTP, Parquet and PostgreSQL boundaries.

use arrow_array::{Array, Date32Array, Int16Array, Int32Array, RecordBatch, StringArray};
use axum::{
    Json, Router,
    extract::{RawQuery, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, NaiveDate, Utc};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgPool, postgres::PgConnectOptions};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
    process::Output,
    time::Duration,
};
use tokio::{net::TcpListener, process::Command, task::JoinHandle};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fixture {
    Original,
    Corrected,
    SourceFailure,
}

struct FixtureServer {
    url: String,
    task: JoinHandle<()>,
}
impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl FixtureServer {
    async fn start(mode: Fixture) -> Self {
        let app = Router::new()
            .route("/api/Members/Search", get(search))
            .route("/api/Members/History", get(histories))
            .with_state(mode);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { url, task }
    }
    async fn stop(&mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}
fn profile(id: i32, mode: Fixture, current_search: bool) -> Value {
    let name = if current_search {
        format!("Current search profile {id}")
    } else if id == 1 && mode == Fixture::Corrected {
        "Corrected Member".into()
    } else {
        format!("Historical Member {id}")
    };
    json!({"value": {"id": id, "nameDisplayAs": name,
        "latestParty": if id == 4 { Value::Null } else { json!({"id": 15, "name": if id == 2 && mode == Fixture::Corrected { "Corrected Party" } else { "Party" }}) },
        "latestHouseMembership": {"house": if matches!(id, 2 | 4) { 2 } else { 1 }, "membershipFrom": if id == 4 { Value::Null } else { json!(" Source location ") }}
    }})
}
async fn search(State(mode): State<Fixture>, RawQuery(query): RawQuery) -> Response {
    let url = reqwest::Url::parse(&format!("http://fixture/?{}", query.unwrap())).unwrap();
    let params: BTreeMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(params["take"], "20");
    let offset: usize = params["skip"].parse().unwrap();
    let current = params.contains_key("IsCurrentMember");
    let ids = if current {
        assert_eq!(params["IsCurrentMember"], "true");
        assert_eq!(params["House"], "1");
        vec![1, 1, 3] // Identical repeated profiles within this stream.
    } else {
        assert!(!params.contains_key("House"));
        assert_eq!(params["MembershipInDateRange.WasMemberOfHouse"], "1");
        assert_eq!(
            params["MembershipInDateRange.WasMemberOnOrAfter"],
            "2024-07-04"
        );
        NaiveDate::parse_from_str(
            &params["MembershipInDateRange.WasMemberOnOrBefore"],
            "%Y-%m-%d",
        )
        .unwrap();
        if mode == Fixture::SourceFailure && offset > 0 {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                [("Retry-After", "0")],
                "fixture unavailable",
            )
                .into_response();
        }
        if mode == Fixture::Corrected {
            vec![1, 2, 1, 3, 4]
        } else {
            vec![1, 2, 1, 3, 4, 5]
        }
    };
    let items: Vec<_> = ids
        .iter()
        .skip(offset)
        .take(2)
        .map(|id| profile(*id, mode, current))
        .collect();
    // Later metadata shrinks the advertised total; clients must re-read it.
    Json(json!({"items": items, "totalResults": if offset == 0 { 9 } else { ids.len() }, "skip": offset, "take": 20})).into_response()
}
fn membership(house: i16, start: &str, end: Option<&str>) -> Value {
    json!({"house": house, "membershipStartDate": start, "membershipEndDate": end})
}
async fn histories(State(mode): State<Fixture>, RawQuery(query): RawQuery) -> Json<Value> {
    let url = reqwest::Url::parse(&format!("http://fixture/?{}", query.unwrap())).unwrap();
    let values: Vec<_> = url
        .query_pairs()
        .map(|(key, id)| {
            assert_eq!(key, "ids");
            let id: i32 = id.parse().unwrap();
            let periods = match id {
                1 => vec![
                    membership(1, "2020-01-01T23:30:00-02:00", None),
                    membership(1, "2020-01-01T23:30:00-02:00", None),
                ],
                2 => vec![
                    membership(
                        1,
                        "2020-01-01",
                        Some(if mode == Fixture::Corrected {
                            "2025-02-02"
                        } else {
                            "2025-01-31T23:30:00-02:00"
                        }),
                    ),
                    membership(2, "2025-02-03", None),
                ],
                3 => vec![
                    membership(1, "2020-01-01", Some("2024-12-01")),
                    membership(
                        1,
                        if mode == Fixture::Corrected {
                            "2025-02-05"
                        } else {
                            "2025-02-01"
                        },
                        None,
                    ),
                ],
                4 => vec![
                    membership(1, "2020-01-01", Some("2024-07-04")),
                    membership(2, "2024-07-05", None),
                ],
                5 => vec![membership(1, "2020-01-01", Some("2024-08-01"))],
                _ => panic!("unexpected member ID {id}"),
            };
            json!({"value": {"id": id, "houseMembershipHistory": periods}})
        })
        .collect();
    // The API may return histories in a different order from requested IDs.
    Json(json!(values.into_iter().rev().collect::<Vec<_>>()))
}
async fn command(config: &Path, operation: &str, capture_id: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_exposed"));
    command
        .args(["data", operation, "members"])
        .arg(config)
        .env_remove("DATABASE_URL");
    if let Some(id) = capture_id {
        command.arg(id);
    }
    tokio::time::timeout(Duration::from_secs(40), command.kill_on_drop(true).output())
        .await
        .expect("CLI timed out")
        .unwrap()
}
fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["status"], "succeeded");
    summary
}
fn write_fetch_config(root: &Path, server: &FixtureServer) -> PathBuf {
    let config = root.join("fetch.yaml");
    fs::write(
        &config,
        format!(
            "data_root: {}\nterm_start: '2024-07-04'\nmembers_api_url: {}\n",
            root.display(),
            server.url
        ),
    )
    .unwrap();
    config
}
fn parquet(path: &Path) -> RecordBatch {
    let mut batches = ParquetRecordBatchReaderBuilder::try_new(File::open(path).unwrap())
        .unwrap()
        .build()
        .unwrap();
    let batch = batches.next().unwrap().unwrap();
    assert!(batches.next().is_none());
    batch
}
fn column<'a, T: Array + 'static>(batch: &'a RecordBatch, name: &str) -> &'a T {
    batch
        .column_by_name(name)
        .unwrap()
        .as_any()
        .downcast_ref()
        .unwrap()
}
fn capture_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_str().unwrap().into(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}
async fn database_state(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object(
        'terms', (SELECT coalesce(jsonb_agg(t ORDER BY term_start), '[]') FROM exposed.parliament_terms t),
        'members', (SELECT coalesce(jsonb_agg(m ORDER BY parliament_member_id), '[]') FROM exposed.members m),
        'services', (SELECT coalesce(jsonb_agg(s ORDER BY member_id, source_start_date), '[]') FROM exposed.member_terms s),
        'declarations', (SELECT coalesce(jsonb_agg(d ORDER BY id), '[]') FROM exposed.declarations d))")
        .fetch_one(pool).await.unwrap()
}
fn member(state: &Value, source_id: i64) -> &Value {
    state["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["parliament_member_id"] == source_id)
        .unwrap()
}
fn services(state: &Value, source_id: i64) -> Vec<&Value> {
    let id = &member(state, source_id)["id"];
    state["services"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| &s["member_id"] == id)
        .collect()
}

// The workflow runs in a task so cleanup also runs if an assertion panics.
#[tokio::test]
async fn captures_and_loads_members_atomically_offline() {
    let admin_url = std::env::var("EXPOSED_TEST_ADMIN_DSN").or_else(|_| std::env::var("DATABASE_URL")).expect("set EXPOSED_TEST_ADMIN_DSN or DATABASE_URL to a local PostgreSQL server with CREATE DATABASE permission");
    let options: PgConnectOptions = admin_url.parse().unwrap();
    let admin = PgPool::connect_with(options.clone().database("postgres"))
        .await
        .unwrap();
    // The SQL identifier contains only a fixed prefix and hexadecimal UUID digits.
    let name = format!("exposed_member_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&admin)
        .await
        .unwrap();
    let database_options = options.database(&name);
    let result = tokio::spawn(async move {
        let pool = PgPool::connect_with(database_options).await.unwrap();
        sqlx::migrate!("../db/migrations").run(&pool).await.unwrap();
        member_workflow(&pool).await;
        pool.close().await;
    })
    .await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {name} WITH (FORCE)"
    )))
    .execute(&admin)
    .await
    .unwrap();
    admin.close().await;
    result.unwrap();
}

async fn member_workflow(pool: &PgPool) {
    let root = tempfile::tempdir().unwrap();
    let mut server = FixtureServer::start(Fixture::Original).await;
    let config = write_fetch_config(root.path(), &server);
    let empty_database = database_state(pool).await;

    // 1. Fetch through the real executable with no database configuration.
    let fetched = success(&command(&config, "fetch", None).await);
    let id = fetched["capture_id"].as_str().unwrap();
    assert_eq!(Uuid::parse_str(id).unwrap().get_version_num(), 7);
    assert_eq!(
        fetched["counts"],
        json!({"profiles": 5, "current_commons": 2, "house_memberships": 9})
    );
    let path = root.path().join("raw/members").join(id);
    let original_files = capture_files(&path);
    assert_eq!(original_files.len(), 4);
    let manifest: Value = serde_json::from_slice(&original_files["manifest.json"]).unwrap();
    assert_eq!(manifest["capture_id"], id);
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["term_start"], "2024-07-04");
    let started: DateTime<Utc> = serde_json::from_value(manifest["started_at"].clone()).unwrap();
    let completed: DateTime<Utc> =
        serde_json::from_value(manifest["completed_at"].clone()).unwrap();
    assert!(completed >= started);
    assert_eq!(
        manifest["observation_date"],
        started
            .with_timezone(&chrono_tz::Europe::London)
            .date_naive()
            .to_string()
    );

    // 2. Inspect all datasets with a real Parquet reader, including nulls/dates.
    let profiles = parquet(&path.join("profiles.parquet"));
    assert_eq!(profiles.num_rows(), 5);
    assert_eq!(
        column::<Int32Array>(&profiles, "parliament_member_id")
            .values()
            .as_ref(),
        &[1, 2, 3, 4, 5]
    );
    assert_eq!(
        column::<StringArray>(&profiles, "name").value(0),
        "Historical Member 1"
    );
    assert_eq!(column::<Int16Array>(&profiles, "latest_house").value(1), 2);
    assert!(column::<Int32Array>(&profiles, "party_id").is_null(3));
    assert!(column::<StringArray>(&profiles, "party_name").is_null(3));
    assert!(column::<StringArray>(&profiles, "latest_membership_from").is_null(3));
    assert_eq!(
        column::<StringArray>(&profiles, "latest_membership_from").value(0),
        " Source location "
    );
    let current = parquet(&path.join("current_commons.parquet"));
    assert_eq!(
        column::<Int32Array>(&current, "parliament_member_id")
            .values()
            .as_ref(),
        &[1, 3]
    );
    let history = parquet(&path.join("house_memberships.parquet"));
    assert_eq!(history.num_rows(), 9);
    assert_eq!(
        column::<Date32Array>(&history, "source_start_date")
            .value_as_date(0)
            .unwrap()
            .to_string(),
        "2020-01-01"
    );
    assert_eq!(
        column::<Date32Array>(&history, "source_end_date")
            .value_as_date(2)
            .unwrap()
            .to_string(),
        "2025-01-31"
    );
    assert!(column::<Date32Array>(&history, "source_end_date").is_null(0));
    assert_eq!(database_state(pool).await, empty_database);

    // 3. Stop Parliament, then load using only the saved observation context.
    server.stop().await;
    assert!(reqwest::get(&server.url).await.is_err());
    let load_config = root.path().join("load.yaml");
    let connection_string = pool.connect_options().to_url_lossy().to_string();
    fs::write(&load_config, format!("data_root: {}\nconnection_string: {connection_string}\nterm_start: '2099-01-01'\nmembers_api_url: invalid-and-unused\n", root.path().display())).unwrap();
    assert_eq!(
        command(&load_config, "load", None).await.status.code(),
        Some(2)
    );
    assert_eq!(
        command(
            &load_config,
            "load",
            Some("00000000-0000-4000-8000-000000000000")
        )
        .await
        .status
        .code(),
        Some(2)
    );
    // Fetch's configuration deliberately lacks the database setting required by Load.
    assert_eq!(
        command(&config, "load", Some(id)).await.status.code(),
        Some(2)
    );
    let loaded = success(&command(&load_config, "load", Some(id)).await);
    assert_eq!(loaded["capture_id"], id);
    assert_eq!(loaded["observation_date"], manifest["observation_date"]);
    assert_eq!(loaded["term_start"], "2024-07-04");
    assert_eq!(
        loaded["summary"],
        json!({"members": 4, "current_commons": 2, "former_commons": 2, "inserted": 4, "updated": 0, "unchanged": 0, "service_periods": 5, "excluded_candidates": 1})
    );
    let first = database_state(pool).await;
    assert_eq!(first["terms"][0]["term_start"], "2024-07-04T00:00:00+00:00");
    assert_eq!(member(&first, 2)["latest_house"], 2);
    assert_eq!(member(&first, 2)["is_current_commons"], false);
    assert_eq!(member(&first, 1)["name"], "Historical Member 1");
    assert_eq!(
        services(&first, 1)[0]["served_from"],
        "2024-07-04T00:00:00+00:00"
    );
    assert_eq!(
        services(&first, 2)[0]["served_until"],
        "2025-01-31T00:00:00+00:00"
    );
    assert_eq!(services(&first, 3).len(), 2);
    assert_eq!(
        services(&first, 3)[0]["served_until"],
        "2024-12-01T00:00:00+00:00"
    );
    assert_eq!(
        services(&first, 3)[1]["served_from"],
        "2025-02-01T00:00:00+00:00"
    );
    for row in first["members"]
        .as_array()
        .unwrap()
        .iter()
        .chain(first["services"].as_array().unwrap())
    {
        assert_eq!(
            Uuid::parse_str(row["id"].as_str().unwrap())
                .unwrap()
                .get_version_num(),
            7
        );
    }

    // 4. Preserve UUIDs, rows and timestamps on repeat, including external data.
    sqlx::query("UPDATE exposed.parliament_terms SET term_end = '2029-07-04T00:00:00Z'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at) SELECT 123, id, 1, 'Employment', now() FROM exposed.members WHERE parliament_member_id = 1").execute(pool).await.unwrap();
    let before_repeat = database_state(pool).await;
    let repeated = success(&command(&load_config, "load", Some(id)).await);
    assert_eq!(repeated["summary"]["unchanged"], 4);
    assert_eq!(repeated["summary"]["inserted"], 0);
    assert_eq!(database_state(pool).await, before_repeat);

    // 5. A later capture corrects profiles, an end date, and a re-entry start.
    let mut corrected_server = FixtureServer::start(Fixture::Corrected).await;
    let config = write_fetch_config(root.path(), &corrected_server);
    let corrected = success(&command(&config, "fetch", None).await);
    let corrected_id = corrected["capture_id"].as_str().unwrap();
    assert!(Uuid::parse_str(corrected_id).unwrap() > Uuid::parse_str(id).unwrap());
    assert_eq!(database_state(pool).await, before_repeat);
    corrected_server.stop().await;
    sqlx::raw_sql("CREATE FUNCTION exposed.reject_fixture_member() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
          IF NEW.parliament_member_id = 2 THEN
            IF (SELECT name FROM exposed.members WHERE parliament_member_id = 1) <> 'Corrected Member' THEN
              RAISE EXCEPTION 'fixture did not reach an earlier write';
            END IF;
            RAISE EXCEPTION 'fixture rejection after earlier write';
          END IF;
          RETURN NEW;
        END; $$;
        CREATE TRIGGER reject_fixture_member BEFORE INSERT OR UPDATE ON exposed.members FOR EACH ROW EXECUTE FUNCTION exposed.reject_fixture_member();")
        .execute(pool).await.unwrap();
    let rejected = command(&load_config, "load", Some(corrected_id)).await;
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("fixture rejection after earlier write")
    );
    assert_eq!(database_state(pool).await, before_repeat);
    sqlx::raw_sql("DROP TRIGGER reject_fixture_member ON exposed.members; DROP FUNCTION exposed.reject_fixture_member();").execute(pool).await.unwrap();
    let corrected_load = success(&command(&load_config, "load", Some(corrected_id)).await);
    assert_eq!(
        corrected_load["summary"],
        json!({"members": 3, "current_commons": 2, "former_commons": 1, "inserted": 0, "updated": 2, "unchanged": 1, "service_periods": 4, "excluded_candidates": 1})
    );
    let after_correction = database_state(pool).await;
    assert_eq!(member(&after_correction, 1)["name"], "Corrected Member");
    assert_eq!(
        member(&after_correction, 2)["party_name"],
        "Corrected Party"
    );
    for source_id in [1, 2, 3, 5] {
        assert_eq!(
            member(&after_correction, source_id)["id"],
            member(&first, source_id)["id"]
        );
    }
    assert_eq!(services(&after_correction, 1), services(&first, 1));
    assert_eq!(
        services(&after_correction, 2)[0]["id"],
        services(&first, 2)[0]["id"]
    );
    assert_eq!(
        services(&after_correction, 2)[0]["served_until"],
        "2025-02-02T00:00:00+00:00"
    );
    assert_eq!(services(&after_correction, 3)[0], services(&first, 3)[0]);
    assert_ne!(
        services(&after_correction, 3)[1]["id"],
        services(&first, 3)[1]["id"]
    );
    assert_eq!(
        services(&after_correction, 3)[1]["source_start_date"],
        "2025-02-05T00:00:00+00:00"
    );
    assert_eq!(member(&after_correction, 5), member(&first, 5));
    assert_eq!(services(&after_correction, 5), services(&first, 5));
    assert_eq!(
        after_correction["declarations"],
        before_repeat["declarations"]
    );
    assert_eq!(after_correction["terms"], before_repeat["terms"]);
    assert_eq!(capture_files(&path), original_files);

    // Explicit older captures remain loadable; the filesystem does not choose.
    let older = success(&command(&load_config, "load", Some(id)).await);
    assert_eq!(older["summary"]["updated"], 2);
    let after_older = database_state(pool).await;
    assert_eq!(member(&after_older, 1)["name"], "Historical Member 1");
    assert_eq!(member(&after_older, 1)["id"], member(&first, 1)["id"]);
    assert_eq!(after_older["declarations"], before_repeat["declarations"]);

    // 6. A later-page source failure exposes no new completed or staging data.
    let completed_before: BTreeSet<_> = fs::read_dir(root.path().join("raw/members"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    let corrected_files = capture_files(&root.path().join("raw/members").join(corrected_id));
    let failing_server = FixtureServer::start(Fixture::SourceFailure).await;
    let config = write_fetch_config(root.path(), &failing_server);
    let failed = command(&config, "fetch", None).await;
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("after four attempts"));
    let completed_after: BTreeSet<_> = fs::read_dir(root.path().join("raw/members"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(completed_after, completed_before);
    assert_eq!(capture_files(&path), original_files);
    assert_eq!(
        capture_files(&root.path().join("raw/members").join(corrected_id)),
        corrected_files
    );
    assert_eq!(database_state(pool).await, after_older);
}
