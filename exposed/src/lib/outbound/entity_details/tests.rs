use crate::{
    domain::{
        models::{funder::FunderId, parliament_member::MemberId},
        repositories::entity_details::EntityDetailsRepo,
        services::entity_details::{EntityDetailsError, EntityDetailsService, Service},
    },
    outbound::postgres::ExposedDatabase,
};
use bigdecimal::BigDecimal;
use sqlx::PgPool;

const FUNDER: &str = "10000000-0000-0000-0000-000000000001";
const ALIASES: [&str; 5] = [
    " Exact Funder ",
    "EXACT FUNDER",
    "Exact Funder",
    "Exact Funder Ltd.",
    "exact funder",
];

#[sqlx::test(migrations = "../db/migrations", fixtures("details"))]
async fn recent_window_preserves_occurrences_and_empty_declarations(pool: PgPool) {
    let db = ExposedDatabase::from(pool);
    let service = Service::new(db);
    let details = service.member("1".parse().unwrap()).await.unwrap();
    assert_eq!(details.declaration_count, 23);
    assert_eq!(details.declaration_limit, 20);
    assert_eq!(
        details
            .declarations
            .iter()
            .map(|declaration| declaration.source_id)
            .collect::<Vec<_>>(),
        (101..=120).rev().collect::<Vec<_>>()
    );
    assert_eq!(details.declarations[0].entries.len(), 10);
    let duplicate = details.declarations[0]
        .entries
        .iter()
        .filter(|entry| {
            entry.amount == Some("9007199254740993.123456789".parse::<BigDecimal>().unwrap())
        })
        .count();
    assert_eq!(duplicate, 2);
    assert!(
        details.declarations[0]
            .entries
            .iter()
            .any(|entry| entry.funder.is_none()
                && entry.amount.is_none()
                && entry.currency.is_none())
    );
    assert!(
        details.declarations[1..]
            .iter()
            .all(|declaration| declaration.entries.is_empty())
    );
    let details = service.member("2".parse().unwrap()).await.unwrap();
    assert_eq!(
        details
            .declarations
            .iter()
            .map(|declaration| declaration.source_id)
            .collect::<Vec<_>>(),
        vec![202, 201, 200]
    );
    assert!(details.declarations[2].registered_at.is_none());
    assert!(details.declarations[0].registered_at.is_some());
    assert!(
        service
            .member("3".parse().unwrap())
            .await
            .unwrap()
            .declarations
            .is_empty()
    );
    assert!(matches!(
        service.member("999".parse().unwrap()).await,
        Err(EntityDetailsError::NotFound)
    ));
    let former_mp = service.member("16".parse().unwrap()).await.unwrap();
    assert_eq!(former_mp.member.membership_from, "Life peer");
    assert!(!former_mp.member.is_current_commons);
}

#[sqlx::test(migrations = "../db/migrations", fixtures("details"))]
async fn exact_summaries_exclude_unknown_currencies_and_keep_local_coverage(pool: PgPool) {
    let db = ExposedDatabase::from(pool);
    let raw = db
        .funder_funding(FUNDER.parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(raw.profile.aliases, ALIASES);
    assert!(
        raw.allocations
            .iter()
            .filter(|allocation| allocation.currency.is_none())
            .all(|allocation| allocation.known_total.is_none())
    );
    let details = Service::new(db)
        .funder(FUNDER.parse().unwrap())
        .await
        .unwrap();
    assert_eq!(details.profile.company_number.as_deref(), Some("00123456"));
    assert_eq!(details.profile.aliases, ALIASES);
    assert_eq!(details.entry_count, 25);
    assert_eq!(details.unknown_currency_count, 5);
    assert_eq!(
        details
            .currencies
            .iter()
            .map(|group| group.currency.as_str())
            .collect::<Vec<_>>(),
        vec!["EUR", "GBP", "USD"]
    );
    let eur = &details.currencies[0];
    assert!(eur.parties[0].amount.is_none());
    assert_eq!(eur.parties[0].unknown_amount_count, 1);
    assert!(eur.top_recipients[0].known_total.is_none());
    let gbp = &details.currencies[1];
    assert_eq!(
        gbp.parties[0].amount,
        Some("18014398509481998.246913578".parse::<BigDecimal>().unwrap())
    );
    assert_eq!(gbp.parties[0].entry_count, 15);
    assert_eq!(gbp.parties[0].unknown_amount_count, 1);
    assert_eq!(
        gbp.parties[1].amount,
        Some("-5".parse::<BigDecimal>().unwrap())
    );
    assert_eq!(gbp.parties[1].unknown_amount_count, 0);
    assert_eq!(gbp.top_recipients.len(), 10);
    assert_eq!(
        gbp.top_recipients[0].known_total,
        Some("18014398509481986.246913578".parse::<BigDecimal>().unwrap())
    );
    assert_eq!(gbp.top_recipients[0].unknown_amount_count, 1);
    assert_eq!(
        gbp.top_recipients
            .iter()
            .map(|allocation| allocation.member.id.value())
            .collect::<Vec<_>>(),
        vec![1, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    );
    assert_eq!(details.currencies[2].top_recipients[0].member.id.value(), 2);
    assert_eq!(details.currencies[2].top_recipients[1].member.id.value(), 1);
    assert_eq!(
        details.currencies[2].top_recipients[1].known_total,
        Some("100".parse::<BigDecimal>().unwrap())
    );
    assert_eq!(
        details.currencies[2].parties[0].amount,
        Some("500".parse::<BigDecimal>().unwrap())
    );
}

#[sqlx::test(migrations = "../db/migrations", fixtures("details"))]
async fn empty_funders_are_distinct_from_missing_identities(pool: PgPool) {
    let service = Service::new(ExposedDatabase::from(pool));
    let details = service
        .funder("10000000-0000-0000-0000-000000000002".parse().unwrap())
        .await
        .unwrap();
    assert_eq!(details.entry_count, 0);
    assert!(details.profile.aliases.is_empty());
    let individual = service
        .funder("10000000-0000-0000-0000-000000000004".parse().unwrap())
        .await
        .unwrap();
    assert_eq!(individual.profile.aliases, ["Individual Funder"]);
    assert!(details.currencies.is_empty());
    assert!(matches!(
        service
            .funder("10000000-0000-0000-0000-000000000003".parse().unwrap())
            .await,
        Err(EntityDetailsError::NotFound)
    ));
}

#[test]
fn path_identities_reject_invalid_domain_values() {
    for id in ["0", "-1", "abc", "2147483648", "4294967296"] {
        assert!(id.parse::<MemberId>().is_err(), "{id}");
    }
    assert_eq!("4514".parse::<MemberId>().unwrap().value(), 4514);
    assert!("4514".parse::<FunderId>().is_err());
    assert_eq!(
        FUNDER.parse::<FunderId>().unwrap().value().to_string(),
        FUNDER
    );
}

#[sqlx::test(migrations = "../db/migrations", fixtures("details"))]
async fn http_handlers_expose_typed_identities_exact_values_and_statuses(pool: PgPool) {
    use crate::{
        domain::services::entity_search,
        inbound::http::{routes::routes, state::AppState},
    };
    use std::sync::Arc;
    let db = ExposedDatabase::from(pool);
    let state = AppState {
        entity_search_service: Arc::new(entity_search::Service::new(db.clone(), db.clone())),
        entity_details_service: Arc::new(Service::new(db.clone())),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, routes().with_state(state))
            .await
            .unwrap();
    });
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{origin}/members/1"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["member"]["id"], "1");
    assert_eq!(body["declarations"].as_array().unwrap().len(), 20);
    let entries = body["declarations"][0]["entries"].as_array().unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry["amount"] == "9007199254740993.123456789")
            .count(),
        2
    );
    assert!(entries.iter().any(|entry| entry["funder"].is_null()));
    let response = client
        .get(format!("{origin}/funders/{FUNDER}"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["funder"]["id"], FUNDER);
    assert_eq!(body["funder"]["aliases"], serde_json::json!(ALIASES));
    for (id, aliases) in [
        (
            "10000000-0000-0000-0000-000000000002",
            serde_json::json!([]),
        ),
        (
            "10000000-0000-0000-0000-000000000004",
            serde_json::json!(["Individual Funder"]),
        ),
    ] {
        let response = client
            .get(format!("{origin}/funders/{id}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let profile: serde_json::Value =
            serde_json::from_str(&response.text().await.unwrap()).unwrap();
        assert_eq!(profile["funder"]["aliases"], aliases);
    }
    assert_eq!(
        body["currencies"][0]["parties"][0]["amount"],
        serde_json::Value::Null
    );
    assert_eq!(
        body["currencies"][1]["parties"][0]["amount"],
        "18014398509481998.246913578"
    );
    let response = client
        .get(format!(
            "{origin}/search?term=Exact%20Funder&max_entries=10&strictness=1"
        ))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["entities"][0]["id"], FUNDER);
    for path in [
        "members/0",
        "members/2147483648",
        "members/nope",
        "funders/not-a-uuid",
    ] {
        assert_eq!(
            client
                .get(format!("{origin}/{path}"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    for path in [
        "members/999",
        "funders/10000000-0000-0000-0000-000000000003",
    ] {
        assert_eq!(
            client
                .get(format!("{origin}/{path}"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::NOT_FOUND
        );
    }
    db.pool().close().await;
    let response = client
        .get(format!("{origin}/members/1"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(response.text().await.unwrap(), "internal server error");
    server.abort();
}
