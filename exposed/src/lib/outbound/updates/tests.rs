use crate::{
    domain::{
        models::updates::RegistrationMonth,
        repositories::updates::UpdatesRepo,
        services::updates::{Service, UpdatesService},
    },
    outbound::ExposedDatabase,
};
use sqlx::PgPool;
#[sqlx::test(migrations = "../db/migrations", fixtures("updates"))]
async fn monthly_snapshot_preserves_boundaries_empty_declarations_unknown_categories_and_canonical_counts(
    pool: PgPool,
) {
    let db = ExposedDatabase::from(pool);
    let now = "2026-10-02T00:00:00Z".parse().unwrap();
    let facts = db
        .declarations(&RegistrationMonth::containing(now))
        .await
        .unwrap();
    assert_eq!(facts.len(), 8);
    assert!(
        !facts
            .iter()
            .any(|fact| [102, 103, 700].contains(&fact.source_id))
    );
    assert!(
        facts
            .iter()
            .any(|fact| fact.source_id == 400 && fact.funders.is_empty())
    );
    let updates = Service::new(db.clone()).updates(now).await.unwrap();
    assert_eq!(updates.members.len(), 6);
    assert_eq!(updates.members[0].identity.id.value(), 1);
    assert_eq!(updates.members[0].declaration_count.get(), 2);
    assert_eq!(updates.companies.len(), 3);
    assert_eq!(updates.companies[0].identity.name, "Preferred company");
    assert_eq!(updates.companies[0].declaration_count.get(), 3);
    assert_eq!(
        updates.companies[0].identity.id.value().to_string(),
        "10000000-0000-0000-0000-000000000001"
    );
    assert!(
        updates
            .companies
            .iter()
            .any(|row| row.identity.name == "Trust root")
    );
    assert!(
        updates
            .companies
            .iter()
            .any(|row| row.identity.name == "Unclassified funder")
    );
    let empty = Service::new(db)
        .updates("2026-12-01T00:00:00Z".parse().unwrap())
        .await
        .unwrap();
    assert!(empty.members.is_empty() && empty.companies.is_empty());
}

#[sqlx::test(migrations = "../db/migrations", fixtures("updates"))]
async fn http_updates_expose_current_month_nonfinancial_activity(pool: PgPool) {
    use crate::{
        domain::services::{entity_details, entity_search, rankings},
        inbound::http::{routes::routes, state::AppState},
    };
    use std::sync::Arc;
    let month = RegistrationMonth::containing(chrono::Utc::now());
    let registered_at =
        time::OffsetDateTime::from_unix_timestamp(month.start().timestamp()).unwrap();
    sqlx::query!(
        "UPDATE exposed.declarations SET registration_date = $1 WHERE source_declaration_id = 100",
        registered_at
    )
    .execute(&pool)
    .await
    .unwrap();
    let db = ExposedDatabase::from(pool);
    let state = AppState {
        entity_search_service: Arc::new(entity_search::Service::new(db.clone(), db.clone())),
        entity_details_service: Arc::new(entity_details::Service::new(db.clone())),
        rankings_service: Arc::new(rankings::Service::new(db.clone())),
        updates_service: Arc::new(Service::new(db)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, routes().with_state(state))
            .await
            .unwrap();
    });
    let response = reqwest::get(format!("{origin}/updates")).await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(
        body["month_start"],
        month
            .start()
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    );
    assert_eq!(
        body["month_end"],
        month
            .end()
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    );
    let company = body["companies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Preferred company")
        .unwrap();
    assert_eq!(company["id"], "10000000-0000-0000-0000-000000000001");
    assert!(company["declaration_count"].as_u64().unwrap() > 0);
    assert!(company["latest_registration"].as_str().is_some());
    for section in ["members", "companies"] {
        for row in body[section].as_array().unwrap() {
            assert_eq!(row.as_object().unwrap().len(), 4);
            assert!(row.get("amount").is_none());
            assert!(row.get("total").is_none());
        }
    }
    server.abort();
}
