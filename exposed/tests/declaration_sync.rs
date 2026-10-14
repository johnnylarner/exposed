use axum::{
    Json, Router,
    extract::{OriginalUri, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Default)]
struct Source {
    records: BTreeMap<u32, Value>,
    changed: BTreeSet<u32>,
    requests: Vec<String>,
    hidden_from_member: BTreeSet<u32>,
    fault: Option<&'static str>,
}
type Shared = Arc<Mutex<Source>>;
async fn interests(State(state): State<Shared>, OriginalUri(uri): OriginalUri) -> Response {
    let mut source = state.lock().unwrap();
    source.requests.push(uri.to_string());
    let url = reqwest::Url::parse(&format!("http://localhost{uri}")).unwrap();
    let query = url.query_pairs().into_owned().collect::<BTreeMap<_, _>>();
    if source.fault == Some("http") {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let skip = query["Skip"].parse::<usize>().unwrap();
    let take = query["Take"].parse::<usize>().unwrap();
    if source.fault == Some("hydrate_http") && query.get("InterestIds").is_some_and(|id| id == "3")
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let discovery = query.contains_key("PublishedFrom") || query.contains_key("UpdatedFrom");
    let mut items = source
        .records
        .iter()
        .filter(|(id, value)| {
            if let Some(member) = query.get("MemberId") {
                !source.hidden_from_member.contains(id)
                    && value["registrant"]["memberDetail"]["id"].as_u64()
                        == Some(member.parse().unwrap())
            } else if let Some(requested) = query.get("InterestIds") {
                **id == requested.parse::<u32>().unwrap()
            } else if query.contains_key("PublishedFrom") {
                source.changed.contains(id) && **id >= 8
            } else {
                source.changed.contains(id) && **id < 8
            }
        })
        .map(|(_, value)| value.clone())
        .collect::<Vec<_>>();
    if discovery {
        for value in &mut items {
            let last = value["versions"].as_array().unwrap()[0].clone();
            value["versions"] = json!([last]);
        }
    }
    if source.fault == Some("hydrate_owner")
        && query.contains_key("InterestIds")
        && !items.is_empty()
    {
        items[0]["registrant"]["memberDetail"]["id"] = json!(2);
    }
    let total = items.len();
    let mut page = items.into_iter().skip(skip).take(take).collect::<Vec<_>>();
    if discovery && source.fault == Some("duplicate") && skip > 0 && !page.is_empty() {
        page[0] = source.records[&1].clone();
    }
    if discovery && source.fault == Some("truncated") && skip > 0 {
        page.clear();
    }
    let total = if discovery && source.fault == Some("total") && skip > 0 {
        total + 1
    } else {
        total
    };
    Json(json!({"items": page, "totalResults": total})).into_response()
}

fn record(id: u32, member: u32, fields: Value, parent: Option<u32>) -> Value {
    json!({"id": id, "parentInterestId": parent, "category": {"id": 3, "name": "Donations"}, "registrant": {"type": "Member", "memberDetail": {"id": member}}, "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "registrationDate": "2026-09-01", "fields": fields}]})
}
fn payer(name: &str) -> Value {
    json!([{"name":"PayerName", "value":name}, {"name":"Value", "value":"10"}])
}
fn company(name: &str, amount: &str) -> Value {
    json!([{"name":"DonorName", "value":name}, {"name":"DonorStatus", "value":"Company"}, {"name":"DonorCompanyIdentifier", "value":"00001234"}, {"name":"Value", "value":amount}])
}
fn child(amount: &str) -> Value {
    json!([{"name":"IsUltimatePayerDifferent", "value":false}, {"name":"Value", "value":amount}])
}

async fn cli(
    pool: &PgPool,
    config: &Path,
    stage: &str,
    resume: Option<Uuid>,
) -> std::process::Output {
    let options = pool.connect_options();
    let mut url = reqwest::Url::parse(&std::env::var("DATABASE_URL").unwrap()).unwrap();
    url.set_path(options.get_database().unwrap());
    let url = url.to_string();
    let config = config.to_owned();
    let stage = stage.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_exposed"));
        command
            .args(["data", "declarations", &stage, "--config"])
            .arg(config)
            .env("DATABASE_URL", url);
        if let Some(run) = resume {
            command.args([
                if matches!(stage.as_str(), "sync" | "reconcile") {
                    "--resume"
                } else {
                    "--ingestion-key"
                },
                &run.to_string(),
            ]);
        }
        command.output().unwrap()
    })
    .await
    .unwrap()
}
fn success(output: std::process::Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
async fn head(pool: &PgPool) -> (Uuid, i64, String) {
    let row = sqlx::query!("SELECT run.ingestion_key, run.revision, run.coverage_date::text AS \"date!\" FROM exposed.declaration_sync_head head JOIN exposed.declaration_sync_runs run USING (ingestion_key)").fetch_one(pool).await.unwrap();
    (row.ingestion_key, row.revision, row.date)
}
async fn api_json(url: &str) -> Value {
    let response = reqwest::get(url).await.unwrap().error_for_status().unwrap();
    serde_json::from_slice(&response.bytes().await.unwrap()).unwrap()
}
async fn projection(pool: &PgPool) -> Value {
    sqlx::query_scalar!(r#"SELECT jsonb_build_object(
        'declarations', (SELECT jsonb_agg(jsonb_build_array(source_declaration_id, member_id, category_id, category_name, registration_date) ORDER BY source_declaration_id) FROM exposed.declarations),
        'entries', (SELECT jsonb_agg(jsonb_build_array(e.source_declaration_id, e.amount, e.currency, e.payment_type, f.funder_name, f.company_number, e.attribution_basis, e.selected_parent_declaration_id, e.unavailable_reason, e.attribution_issues) ORDER BY e.source_declaration_id, e.source_funding_entry_id) FROM exposed.funding_entries e LEFT JOIN exposed.funders f ON f.id = e.funder_id),
        'aliases', (SELECT jsonb_agg(jsonb_build_array(f.company_number, a.funder_alias) ORDER BY f.company_number, a.funder_alias) FROM exposed.funder_aliases a JOIN exposed.funders f ON f.id = a.funder_id),
        'groups', (SELECT jsonb_agg(jsonb_build_array(f.funder_name, g.group_id = g.funder_id) ORDER BY f.funder_name) FROM exposed.funder_alias_groups g JOIN exposed.funders f ON f.id = g.funder_id)
    ) AS "projection!""#).fetch_one(pool).await.unwrap()
}
fn set_cutoff(config: &Path, cutoff: &str) {
    let text = std::fs::read_to_string(config).unwrap();
    let text = text
        .lines()
        .map(|line| {
            if line.starts_with("cutoff:") {
                format!("cutoff: {cutoff}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(config, text).unwrap();
}
fn copy_directory(source: &Path, destination: &Path) {
    std::fs::create_dir(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_directory(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[sqlx::test(
    migrations = "../db/migrations",
    fixtures("../../db/fixtures/add_members.sql")
)]
async fn declaration_sync_cli_acceptance(pool: PgPool) {
    let state = Shared::default();
    {
        let mut source = state.lock().unwrap();
        source
            .records
            .insert(1, record(1, 1, payer("Original Parent"), None));
        source.records.insert(2, record(2, 1, child("20"), Some(1)));
        let mut multiple = company("Earlier Company", "100")
            .as_array()
            .unwrap()
            .clone();
        multiple.push(json!({"name":"Donors", "values":[[{"name":"Name", "value":"Removed Donor"}, {"name":"Value", "value":"90"}], [{"name":"Name", "value":"Retained Nested"}, {"name":"Value", "value":"30"}]]}));
        source
            .records
            .insert(3, record(3, 1, json!(multiple), None));
        source
            .records
            .insert(4, record(4, 1, company("Historical Company", "50"), None));
        source
            .records
            .insert(5, record(5, 1, child("7"), Some(999)));
        source.records.insert(6, record(6, 1, child("8"), Some(77)));
        source
            .records
            .insert(77, record(77, 9, payer("Outside parent"), None));
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let application = Router::new()
        .route("/Interests", axum::routing::get(interests))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, application).await.unwrap();
    });
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let config = temporary.path().join("sync.yaml");
    std::fs::write(&config, format!("data_dir: {}\nbatch_size: 1\ncandidate_budget: 100000\ncutoff: 2026-10-12\ninterests_url: http://{address}/Interests\n", data.display())).unwrap();
    assert!(!cli(&pool, &config, "sync", None).await.status.success());
    set_cutoff(&config, "2099-01-01");
    assert!(
        !cli(&pool, &config, "reconcile", None)
            .await
            .status
            .success()
    );
    set_cutoff(&config, "2026-10-12");
    assert!(state.lock().unwrap().requests.is_empty());
    success(cli(&pool, &config, "reconcile", None).await);
    let baseline = head(&pool).await;
    let baseline_manifest: Value = serde_json::from_slice(
        &std::fs::read(data.join(baseline.0.to_string()).join("capture.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(baseline_manifest["scope"], json!([1, 2]));
    let original_child_time =
        baseline_manifest["declarations"]["2"]["declaration"]["fetched_at"].clone();
    {
        let mut source = state.lock().unwrap();
        source.requests.clear();
        source
            .records
            .insert(1, record(1, 1, payer("Corrected Parent"), None));
        let mut corrected_fields = company("Current Company", "40").as_array().unwrap().clone();
        corrected_fields.push(json!({"name":"Donors", "values":[[{"name":"Name", "value":"Retained Nested"}, {"name":"Value", "value":"30"}]]}));
        let mut corrected = record(3, 1, json!(corrected_fields), None);
        let old =
            record(3, 1, company("Older Published Company", "999"), None)["versions"][0].clone();
        let mut old = old;
        old["register"] = json!({"id": 700, "publishedDate":"2025-01-01"});
        corrected["versions"].as_array_mut().unwrap().push(old);
        source.records.insert(3, corrected);
        source
            .records
            .insert(8, record(8, 2, company("New Company Alias", "60"), None));
        source
            .records
            .insert(9, record(9, 9, payer("Excluded outsider"), None));
        source.changed = [1, 3, 8, 9].into_iter().collect();
    }
    set_cutoff(&config, "2026-10-13");
    success(cli(&pool, &config, "sync", None).await);
    let delta = head(&pool).await;
    assert_eq!(delta.2, "2026-10-13");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(data.join(delta.0.to_string()).join("capture.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["declarations"]["2"]["declaration"]["fetched_at"],
        original_child_time
    );
    assert_eq!(
        serde_json::from_str::<Value>(
            manifest["declarations"]["3"]["declaration"]["source_json"]
                .as_str()
                .unwrap()
        )
        .unwrap()["versions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(manifest["exclusions"]["9"], 9);
    {
        let source = state.lock().unwrap();
        assert!(
            source
                .requests
                .iter()
                .all(|request| !request.contains("MemberId="))
        );
        for kind in ["Published", "Updated"] {
            let request = source
                .requests
                .iter()
                .find(|request| request.contains(&format!("{kind}From=")))
                .unwrap();
            assert!(request.contains(&format!("{kind}From=2026-10-05")));
            assert!(request.contains(&format!("{kind}To=2026-10-13")));
        }
        let hydrated = source
            .requests
            .iter()
            .filter_map(|request| {
                reqwest::Url::parse(&format!("http://localhost{request}"))
                    .unwrap()
                    .query_pairs()
                    .find(|(key, _)| key == "InterestIds")
                    .map(|(_, value)| value.into_owned())
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            hydrated,
            ["1", "3", "8", "77", "999"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
    }
    let fixed = sqlx::query!("SELECT e.source_declaration_id, e.amount::text AS amount, f.funder_name AS \"funder_name?\", e.unavailable_reason FROM exposed.funding_entries e LEFT JOIN exposed.funders f ON f.id = e.funder_id ORDER BY e.source_declaration_id").fetch_all(&pool).await.unwrap();
    assert_eq!(
        fixed
            .iter()
            .find(|row| row.source_declaration_id == 2)
            .unwrap()
            .funder_name
            .as_deref(),
        Some("Corrected Parent")
    );
    assert_eq!(
        fixed
            .iter()
            .filter(|row| row.source_declaration_id == 3)
            .count(),
        2
    );
    assert_eq!(
        fixed
            .iter()
            .find(|row| row.source_declaration_id == 3 && row.amount.as_deref() == Some("40"))
            .unwrap()
            .amount
            .as_deref(),
        Some("40")
    );
    assert!(
        !fixed
            .iter()
            .any(|row| row.amount.as_deref() == Some("90") || row.amount.as_deref() == Some("999"))
    );
    for id in [5, 6] {
        assert!(
            fixed
                .iter()
                .find(|row| row.source_declaration_id == id)
                .unwrap()
                .unavailable_reason
                .is_some()
        );
    }
    let aliases = sqlx::query_scalar!(
        "SELECT funder_alias FROM exposed.funder_aliases ORDER BY funder_alias"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(aliases.contains(&"Historical Company".into()));
    assert!(!aliases.contains(&"Older Published Company".into()));
    let app_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let app_address = app_listener.local_addr().unwrap();
    let mut connection = reqwest::Url::parse(&std::env::var("DATABASE_URL").unwrap()).unwrap();
    connection.set_path(pool.connect_options().get_database().unwrap());
    let app_config = exposed::inbound::http::config::ServerConfig {
        connection_string: connection.to_string(),
    };
    let app = tokio::spawn(async move {
        exposed::inbound::http::server::serve_exposed(&app_config, app_listener)
            .await
            .unwrap();
    });
    let search = api_json(&format!(
        "http://{app_address}/search?term=Historical%20Company&strictness=1&max_entries=10"
    ))
    .await;
    let entity = search["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entity| entity["kind"] == "Funder")
        .unwrap();
    assert_eq!(entity["name"], "Current Company");
    assert_eq!(
        entity["match_source"],
        json!({"kind":"alias", "name":"Historical Company"})
    );
    let details = api_json(&format!(
        "http://{app_address}/funders/{}",
        entity["id"].as_str().unwrap()
    ))
    .await;
    assert_eq!(details["entry_count"], 3);
    assert!(
        details["funder"]["aliases"]
            .as_array()
            .unwrap()
            .contains(&json!("Historical Company"))
    );
    let member = api_json(&format!("http://{app_address}/members/1")).await;
    let child = member["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|declaration| declaration["source_id"] == 2)
        .unwrap();
    assert_eq!(child["entries"][0]["funder"]["name"], "Corrected Parent");
    assert_eq!(child["entries"][0]["amount"], "20");
    let corrected = member["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|declaration| declaration["source_id"] == 3)
        .unwrap();
    let amounts = corrected["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["amount"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(amounts, ["30", "40"].into_iter().collect());
    app.abort();
    let expected = projection(&pool).await;
    let ids_before = sqlx::query_scalar!("SELECT id FROM exposed.funders ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    state.lock().unwrap().requests.clear();
    success(cli(&pool, &config, "sync", Some(delta.0)).await);
    assert_eq!(head(&pool).await, delta);
    assert!(state.lock().unwrap().requests.is_empty());
    let legacy = cli(&pool, &config, "load", Some(delta.0)).await;
    assert!(!legacy.status.success());
    assert!(String::from_utf8_lossy(&legacy.stderr).contains("sync owns the projection"));
    state.lock().unwrap().changed.clear();
    set_cutoff(&config, "2026-10-14");
    success(cli(&pool, &config, "sync", None).await);
    let mut empty = head(&pool).await;
    assert_eq!(empty.1, delta.1 + 1);
    assert_eq!(empty.2, "2026-10-14");
    set_cutoff(&config, "2026-10-13");
    assert!(!cli(&pool, &config, "sync", None).await.status.success());
    assert_eq!(head(&pool).await, empty);
    assert_eq!(projection(&pool).await, expected);
    set_cutoff(&config, "2026-10-14");
    assert_eq!(projection(&pool).await, expected);
    assert_eq!(
        sqlx::query_scalar!("SELECT id FROM exposed.funders ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap(),
        ids_before
    );
    assert!(!data.join(empty.0.to_string()).join("resolved").exists());
    for fault in [
        "http",
        "duplicate",
        "truncated",
        "total",
        "hydrate_http",
        "hydrate_owner",
    ] {
        {
            let mut source = state.lock().unwrap();
            source.fault = Some(fault);
            source.changed = [1, 3, 8, 9].into_iter().collect();
        }
        assert!(
            !cli(&pool, &config, "sync", None).await.status.success(),
            "fault {fault} must fail"
        );
        assert_eq!(head(&pool).await, empty);
        assert_eq!(projection(&pool).await, expected);
    }
    state.lock().unwrap().fault = None;
    success(cli(&pool, &config, "sync", None).await);
    let equal = head(&pool).await;
    assert_eq!(equal.1, empty.1 + 1);
    assert_eq!(projection(&pool).await, expected);
    assert_eq!(
        sqlx::query_scalar!("SELECT id FROM exposed.funders ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap(),
        ids_before
    );
    assert!(!data.join(equal.0.to_string()).join("resolved").exists());
    empty = equal;
    let owner_before = state.lock().unwrap().records[&1]["registrant"].clone();
    state.lock().unwrap().records.get_mut(&1).unwrap()["registrant"]["memberDetail"]["id"] =
        json!(9);
    assert!(!cli(&pool, &config, "sync", None).await.status.success());
    assert_eq!(head(&pool).await, empty);
    state.lock().unwrap().records.get_mut(&1).unwrap()["registrant"] = Value::Null;
    assert!(!cli(&pool, &config, "sync", None).await.status.success());
    assert_eq!(head(&pool).await, empty);
    state.lock().unwrap().records.get_mut(&1).unwrap()["registrant"] = owner_before;
    let stale = Uuid::now_v7();
    copy_directory(
        &data.join(empty.0.to_string()),
        &data.join(stale.to_string()),
    );
    let path = data.join(stale.to_string()).join("capture.json");
    let mut stale_manifest: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    stale_manifest["run"] = json!(stale);
    stale_manifest["content_changed"] = json!(true);
    let bytes = serde_json::to_vec_pretty(&stale_manifest).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    use sha2::{Digest, Sha256};
    std::fs::write(
        path.with_file_name("capture.sha256"),
        format!("{:x}", Sha256::digest(&bytes)),
    )
    .unwrap();
    assert!(
        !cli(&pool, &config, "sync", Some(stale))
            .await
            .status
            .success()
    );
    assert_eq!(head(&pool).await, empty);
    assert_eq!(projection(&pool).await, expected);
    success(cli(&pool, &config, "sync", Some(delta.0)).await);
    assert_eq!(head(&pool).await, empty);
    let false_noop = Uuid::now_v7();
    copy_directory(
        &data.join(baseline.0.to_string()),
        &data.join(false_noop.to_string()),
    );
    let false_path = data.join(false_noop.to_string()).join("capture.json");
    let mut false_manifest = baseline_manifest.clone();
    false_manifest["run"] = json!(false_noop);
    false_manifest["expected_revision"] = json!(empty.1);
    false_manifest["content_changed"] = json!(false);
    let false_bytes = serde_json::to_vec_pretty(&false_manifest).unwrap();
    std::fs::write(&false_path, &false_bytes).unwrap();
    std::fs::write(
        false_path.with_file_name("capture.sha256"),
        format!("{:x}", Sha256::digest(&false_bytes)),
    )
    .unwrap();
    let false_output = cli(&pool, &config, "sync", Some(false_noop)).await;
    assert!(!false_output.status.success());
    assert!(
        String::from_utf8_lossy(&false_output.stderr)
            .contains("unchanged publication content differs")
    );
    assert_eq!(head(&pool).await, empty);
    assert_eq!(projection(&pool).await, expected);
    state.lock().unwrap().records.remove(&4);
    success(cli(&pool, &config, "reconcile", None).await);
    assert_eq!(projection(&pool).await, expected);
    state
        .lock()
        .unwrap()
        .records
        .insert(4, record(4, 1, company("Historical Company", "50"), None));
    let member_rows = sqlx::query!("SELECT parliament_member_id, name, party_name FROM exposed.members ORDER BY parliament_member_id").fetch_all(&pool).await.unwrap();
    assert_eq!(member_rows.len(), 2);
    assert_eq!(member_rows[0].name, "John McDonnell");
    assert_eq!(member_rows[1].party_name, "Conservative");
    sqlx::query!("TRUNCATE exposed.declaration_sync_head, exposed.declaration_sync_runs, exposed.declaration_load_runs, exposed.funding_entries, exposed.declarations, exposed.funder_aliases, exposed.funders CASCADE").execute(&pool).await.unwrap();
    success(cli(&pool, &config, "reconcile", None).await);
    assert_eq!(
        projection(&pool).await,
        expected,
        "baseline plus delta must equal complete final evidence"
    );
    let before_recovery = head(&pool).await;
    state
        .lock()
        .unwrap()
        .records
        .insert(1, record(1, 1, payer("Recovery Parent"), None));
    let original_capture_time = sqlx::query_scalar!(
        "SELECT fetched_at FROM exposed.declarations WHERE source_declaration_id = 1"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE exposed.declarations SET fetched_at = '2099-01-01' WHERE source_declaration_id = 1"
    )
    .execute(&pool)
    .await
    .unwrap();
    let failed = cli(&pool, &config, "sync", None).await;
    assert!(!failed.status.success());
    assert_eq!(head(&pool).await, before_recovery);
    assert_eq!(projection(&pool).await, expected);
    let stderr = String::from_utf8_lossy(&failed.stderr);
    let run: Uuid = stderr
        .split("sealed declaration run ")
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let run_path = data.join(run.to_string());
    assert!(run_path.join("cleaned/declarations").exists());
    std::fs::remove_file(run_path.join("cleaned-sha256.json")).unwrap();
    sqlx::query!(
        "UPDATE exposed.declarations SET fetched_at = $1 WHERE source_declaration_id = 1",
        original_capture_time
    )
    .execute(&pool)
    .await
    .unwrap();
    state.lock().unwrap().requests.clear();
    success(cli(&pool, &config, "sync", Some(run)).await);
    assert_eq!(head(&pool).await.0, run);
    assert!(state.lock().unwrap().requests.is_empty());
    let parent = sqlx::query_scalar!("SELECT f.funder_name FROM exposed.funding_entries e JOIN exposed.funders f ON f.id = e.funder_id WHERE e.source_declaration_id = 2").fetch_one(&pool).await.unwrap();
    assert_eq!(parent, "Recovery Parent");
    assert!(run_path.join("cleaned-sha256.json").exists());
    let raw_path = run_path.join("raw/declarations/1.parquet");
    let original_raw = std::fs::read(&raw_path).unwrap();
    std::fs::write(&raw_path, b"broken").unwrap();
    assert!(
        !cli(&pool, &config, "sync", Some(run))
            .await
            .status
            .success()
    );
    assert_eq!(head(&pool).await.0, run);
    std::fs::write(&raw_path, original_raw).unwrap();
    server.abort();
}

#[sqlx::test(
    migrations = "../db/migrations",
    fixtures("../../db/fixtures/add_members.sql")
)]
async fn declaration_scope_requires_explicit_complete_reconciliation(pool: PgPool) {
    let state = Shared::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let application = Router::new()
        .route("/Interests", axum::routing::get(interests))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, application).await.unwrap();
    });
    let temporary = tempfile::tempdir().unwrap();
    let config = temporary.path().join("sync.yaml");
    let data = temporary.path().join("data");
    std::fs::write(&config, format!("data_dir: {}\nbatch_size: 2\ncandidate_budget: 100000\ncutoff: 2026-10-12\ninterests_url: http://{address}/Interests\n", data.display())).unwrap();
    sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at) SELECT 1000, id, 3, 'Legacy', now() FROM exposed.members WHERE parliament_member_id = 1").execute(&pool).await.unwrap();
    let missing = cli(&pool, &config, "reconcile", None).await;
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("complete raw evidence is unavailable")
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM exposed.declaration_sync_head")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM exposed.declarations WHERE source_declaration_id = 1000").fetch_one(&pool).await.unwrap(), 1);
    {
        let mut source = state.lock().unwrap();
        source
            .records
            .insert(1000, record(1000, 1, child("25"), Some(2000)));
        source
            .records
            .insert(2000, record(2000, 1, payer("Hydrated Parent"), None));
        source.hidden_from_member.insert(2000);
    }
    success(cli(&pool, &config, "reconcile", None).await);
    let selected = sqlx::query_scalar!("SELECT f.funder_name FROM exposed.funding_entries e JOIN exposed.funders f ON f.id = e.funder_id WHERE e.source_declaration_id = 1000").fetch_one(&pool).await.unwrap();
    assert_eq!(selected, "Hydrated Parent");
    assert!(
        state
            .lock()
            .unwrap()
            .requests
            .iter()
            .any(|request| request.contains("InterestIds=2000"))
    );
    sqlx::query!("INSERT INTO exposed.members (parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) VALUES (3, 'New Member', 1, 'Unchanged Party', 1, 'New Constituency', true)").execute(&pool).await.unwrap();
    {
        let mut source = state.lock().unwrap();
        source.requests.clear();
        source.records.insert(
            1001,
            record(1001, 3, company("New scope company", "80"), None),
        );
        source.changed.insert(1001);
    }
    let daily = cli(&pool, &config, "sync", None).await;
    assert!(
        String::from_utf8_lossy(&daily.stdout)
            .contains("Stored members awaiting reconciliation: 1")
    );
    success(daily);
    assert_eq!(sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM exposed.declarations WHERE source_declaration_id = 1001").fetch_one(&pool).await.unwrap(), 0);
    assert!(
        state
            .lock()
            .unwrap()
            .requests
            .iter()
            .all(|request| !request.contains("MemberId=") && !request.contains("InterestIds=1001"))
    );
    let daily_head = head(&pool).await;
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(data.join(daily_head.0.to_string()).join("capture.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["scope"], json!([1, 2]));
    assert_eq!(manifest["pending_members"], json!([3]));
    assert_eq!(manifest["exclusions"]["1001"], json!(3));
    success(cli(&pool, &config, "reconcile", None).await);
    assert_eq!(sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM exposed.declarations WHERE source_declaration_id = 1001").fetch_one(&pool).await.unwrap(), 1);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT party_name FROM exposed.members WHERE parliament_member_id = 3"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "Unchanged Party"
    );
    server.abort();
}

#[sqlx::test(
    migrations = "../db/migrations",
    fixtures("../../db/fixtures/add_members.sql")
)]
async fn bootstrap_resume_rejects_an_intervening_legacy_load(pool: PgPool) {
    use chrono::{NaiveDate, Utc};
    use exposed::{
        domain::{
            models::{
                declaration_ingestion::{CapturedDeclaration, CapturedFundingEntry, DeclarationId},
                entity_ingestion::IngestionKey,
                parliament_member::MemberId,
            },
            repositories::entity_ingestion::EntityIngestionStorage,
        },
        outbound::ExposedDataPipeline,
    };

    let state = Shared::default();
    state
        .lock()
        .unwrap()
        .records
        .insert(1, record(1, 1, payer("Initial payer"), None));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let application = Router::new()
        .route("/Interests", axum::routing::get(interests))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, application).await.unwrap();
    });
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let config = temporary.path().join("sync.yaml");
    std::fs::write(&config, format!("data_dir: {}\nbatch_size: 2\ncandidate_budget: 100000\ncutoff: 2026-10-12\ninterests_url: http://{address}/Interests\n", data.display())).unwrap();
    sqlx::query!("CREATE FUNCTION exposed.reject_initial_sync() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected publication failure'; END $$").execute(&pool).await.unwrap();
    sqlx::query!("CREATE TRIGGER reject_initial_sync BEFORE INSERT ON exposed.declaration_sync_runs FOR EACH ROW EXECUTE FUNCTION exposed.reject_initial_sync()").execute(&pool).await.unwrap();
    let failed = cli(&pool, &config, "reconcile", None).await;
    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("injected publication failure"), "{stderr}");
    let run: Uuid = stderr
        .split("sealed declaration run ")
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM exposed.declaration_sync_head")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    sqlx::query!("DROP TRIGGER reject_initial_sync ON exposed.declaration_sync_runs")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("DROP FUNCTION exposed.reject_initial_sync()")
        .execute(&pool)
        .await
        .unwrap();

    let legacy_run = IngestionKey::default();
    let storage = ExposedDataPipeline::new_with_ingestion_key(&data, legacy_run.clone()).unwrap();
    let legacy = CapturedDeclaration::new(
        DeclarationId::new(2).unwrap(),
        None,
        3,
        "Donations".into(),
        820,
        NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
        Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
        vec![CapturedFundingEntry::new(
            None,
            None,
            Some("Intervening legacy payer".into()),
            Some("10".into()),
            None,
            None,
            None,
            None,
            None,
        )],
        Utc::now(),
        record(2, 1, payer("Intervening legacy payer"), None).to_string(),
    )
    .unwrap();
    storage
        .write_raw_declarations(MemberId::new(1).unwrap(), &[legacy])
        .await
        .unwrap();
    for stage in ["clean", "resolve", "load"] {
        success(cli(&pool, &config, stage, Some(legacy_run.uuid())).await);
    }
    let before_resume = projection(&pool).await;
    state.lock().unwrap().requests.clear();
    let resumed = cli(&pool, &config, "sync", Some(run)).await;
    assert!(
        !resumed.status.success(),
        "bootstrap must reject a declaration absent from its sealed evidence"
    );
    assert!(
        String::from_utf8_lossy(&resumed.stderr)
            .contains("initial publication omitted current declarations")
    );
    assert_eq!(projection(&pool).await, before_resume);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM exposed.declaration_sync_head")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(state.lock().unwrap().requests.is_empty());
    server.abort();
}
