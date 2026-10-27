use crate::{
    domain::{
        models::{
            entity_details::{FunderDetailsRequest, MemberDetailsRequest},
            funder::FunderId,
            parliament_member::MemberId,
            reporting_period::ReportingPeriod,
        },
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
async fn complete_history_preserves_occurrences_and_empty_declarations(pool: PgPool) {
    let db = ExposedDatabase::from(pool);
    let service = Service::new(db);
    let details = service
        .member(MemberDetailsRequest::new(
            "1".parse().unwrap(),
            ReportingPeriod::Historical,
        ))
        .await
        .unwrap();
    assert_eq!(details.declarations.len(), 23);
    assert_eq!(
        details
            .declarations
            .iter()
            .map(|declaration| declaration.source_id)
            .collect::<Vec<_>>(),
        (100..=120).rev().chain([122, 121]).collect::<Vec<_>>()
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
        details.declarations[1..16]
            .iter()
            .all(|declaration| declaration.entries.is_empty())
    );
    let details = service
        .member(MemberDetailsRequest::new(
            "2".parse().unwrap(),
            ReportingPeriod::Historical,
        ))
        .await
        .unwrap();
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
            .member(MemberDetailsRequest::new(
                "3".parse().unwrap(),
                ReportingPeriod::Historical
            ))
            .await
            .unwrap()
            .declarations
            .is_empty()
    );
    assert!(matches!(
        service
            .member(MemberDetailsRequest::new(
                "999".parse().unwrap(),
                ReportingPeriod::Historical
            ))
            .await,
        Err(EntityDetailsError::NotFound)
    ));
    let former_mp = service
        .member(MemberDetailsRequest::new(
            "16".parse().unwrap(),
            ReportingPeriod::Historical,
        ))
        .await
        .unwrap();
    assert_eq!(former_mp.member.membership_from, "Life peer");
    assert!(!former_mp.member.is_current_commons);
}

#[sqlx::test(migrations = "../db/migrations", fixtures("details"))]
async fn member_summary_covers_complete_history_and_classifies_by_source_id(pool: PgPool) {
    use crate::domain::models::entity_details::DeclarationCategory;
    let service = Service::new(ExposedDatabase::from(pool));
    let details = service
        .member(MemberDetailsRequest::new(
            "1".parse().unwrap(),
            ReportingPeriod::Historical,
        ))
        .await
        .unwrap();
    assert_eq!(details.summary.entry_count, 18);
    assert_eq!(details.summary.unknown_currency_count, 7);
    assert_eq!(details.summary.undated_declaration_count, 2);
    assert_eq!(
        details.declarations[22].category,
        DeclarationCategory::Other(987)
    );
    assert_eq!(details.declarations[22].category.source_id(), 987);
    assert_eq!(
        details
            .summary
            .currencies
            .iter()
            .map(|group| group.currency.as_str())
            .collect::<Vec<_>>(),
        ["EUR", "GBP", "JPY", "USD"]
    );
    let gbp = &details.summary.currencies[1];
    assert_eq!(
        gbp.amount,
        Some("18014398509481983.996913578".parse::<BigDecimal>().unwrap())
    );
    let support = gbp
        .categories
        .iter()
        .find(|category| category.category.is_donation_support())
        .unwrap();
    assert_eq!(support.amount, Some(BigDecimal::from(0)));
    assert_eq!(support.entry_count, 4);
    let unknown = gbp
        .categories
        .iter()
        .find(|category| category.category == DeclarationCategory::Other(987))
        .unwrap();
    assert_eq!(unknown.category_name, "Unrecognized source category");
    assert_eq!(unknown.amount, Some("-2.25".parse::<BigDecimal>().unwrap()));
    assert!(details.summary.currencies[0].amount.is_none());
    assert_eq!(details.summary.currencies[0].unknown_amount_count, 1);
    assert_eq!(
        details.summary.currencies[2].amount,
        Some(BigDecimal::from(0))
    );
    assert_eq!(
        details.summary.currencies[3].amount,
        Some(BigDecimal::from(100))
    );
    let empty = service
        .member(MemberDetailsRequest::new(
            "3".parse().unwrap(),
            ReportingPeriod::Historical,
        ))
        .await
        .unwrap();
    assert!(empty.summary.currencies.is_empty());
    assert_eq!(empty.summary.entry_count, 0);
    assert!(empty.summary.first_registered_at.is_none());
}

#[sqlx::test(migrations = "../db/migrations", fixtures("details"))]
async fn exact_summaries_exclude_unknown_currencies_and_keep_local_coverage(pool: PgPool) {
    sqlx::query!("INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency, payment_type) VALUES (120, '10000000-0000-0000-0000-000000000004', 999999, 'GBP', 'Other funder')").execute(&pool).await.unwrap();
    let db = ExposedDatabase::from(pool);
    let raw = db
        .funder_funding(FUNDER.parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(raw.profile.aliases, ALIASES);
    assert_eq!(raw.declarations.len(), 14);
    let source = &raw.declarations[0];
    assert_eq!(source.member.id.value(), 1);
    assert_eq!(source.declaration.source_id, 120);
    assert_eq!(source.declaration.entries.len(), 9);
    assert_eq!(
        source.declaration.entries[0].amount,
        source.declaration.entries[1].amount
    );
    assert!(
        source
            .declaration
            .entries
            .iter()
            .all(|entry| entry.funder.as_ref().unwrap().id == FUNDER.parse().unwrap())
    );
    assert_eq!(
        source
            .declaration
            .entries
            .iter()
            .filter(|entry| entry.currency.is_none())
            .count(),
        5
    );
    assert!(
        source
            .declaration
            .entries
            .iter()
            .any(|entry| entry.currency.is_none() && entry.amount == Some(BigDecimal::from(999)))
    );
    let details = Service::new(db)
        .funder(FunderDetailsRequest::new(
            FUNDER.parse().unwrap(),
            ReportingPeriod::Historical,
        ))
        .await
        .unwrap();
    assert_eq!(details.profile.company_number.as_deref(), Some("00123456"));
    assert_eq!(details.profile.aliases, ALIASES);
    assert_eq!(details.entry_count, 25);
    assert_eq!(details.unknown_currency_count, 5);
    assert_funder_declared_totals(&details);
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
        gbp.amount,
        Some("18014398509481993.246913578".parse::<BigDecimal>().unwrap())
    );
    assert!(eur.amount.is_none());
    assert_eq!(
        details
            .declarations
            .iter()
            .map(|source| source.declaration.entries.len())
            .sum::<usize>(),
        25
    );
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
        .funder(FunderDetailsRequest::new(
            "10000000-0000-0000-0000-000000000002".parse().unwrap(),
            ReportingPeriod::Historical,
        ))
        .await
        .unwrap();
    assert_eq!(details.entry_count, 0);
    assert_eq!(details.profile.aliases, ["Empty Funder"]);
    let individual = service
        .funder(FunderDetailsRequest::new(
            "10000000-0000-0000-0000-000000000004".parse().unwrap(),
            ReportingPeriod::Historical,
        ))
        .await
        .unwrap();
    assert_eq!(individual.profile.aliases, ["Individual Funder"]);
    assert!(details.currencies.is_empty());
    assert!(matches!(
        service
            .funder(FunderDetailsRequest::new(
                "10000000-0000-0000-0000-000000000003".parse().unwrap(),
                ReportingPeriod::Historical
            ))
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
        updates_service: std::sync::Arc::new(crate::domain::services::updates::Service::new(
            db.clone(),
        )),
        rankings_service: std::sync::Arc::new(crate::domain::services::rankings::Service::new(
            db.clone(),
        )),
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
        .get(format!("{origin}/members/1?period=historical"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["member"]["id"], "1");
    assert_eq!(body["declarations"].as_array().unwrap().len(), 23);
    assert_eq!(body["declarations"][0]["category_id"], 1);
    assert!(body.get("recent_support").is_none());
    assert_eq!(
        body["summary"]["currencies"][1]["amount"],
        "18014398509481983.996913578"
    );
    assert_eq!(
        body["summary"]["currencies"][0]["amount"],
        serde_json::Value::Null
    );
    assert_eq!(body["summary"]["currencies"][2]["amount"], "0");
    assert!(body.get("declaration_limit").is_none());

    let entries = body["declarations"][0]["entries"].as_array().unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry["amount"] == "9007199254740993.123456789")
            .count(),
        2
    );
    assert!(entries.iter().any(|entry| entry["funder"].is_null()));
    assert!(
        entries
            .iter()
            .any(|entry| entry["amount"] == "888" && entry["currency"].is_null())
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry["amount"] == "100" && entry["currency"] == "USD")
    );
    let response = client
        .get(format!("{origin}/funders/{FUNDER}?period=historical"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["funder"]["id"], FUNDER);
    assert_funder_declared_contract(&body);
    assert_eq!(body["funder"]["aliases"], serde_json::json!(ALIASES));
    assert_eq!(body["funder"]["member_ids"], serde_json::json!([FUNDER]));
    assert_eq!(body["funder"]["alias_links"], serde_json::json!([]));
    assert_eq!(body["funder"]["family_links"], serde_json::json!([]));
    for (id, aliases) in [
        (
            "10000000-0000-0000-0000-000000000002",
            serde_json::json!(["Empty Funder"]),
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
    assert_eq!(
        body["currencies"][1]["amount"],
        "18014398509481993.246913578"
    );
    assert_eq!(body["declarations"][0]["declaration"]["source_id"], 120);
    assert_eq!(body["declarations"][0]["member"]["id"], "1");
    assert_eq!(
        body["declarations"][0]["declaration"]["entries"]
            .as_array()
            .unwrap()
            .len(),
        9
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
    let context = crate::domain::models::reporting_period::ReportingContext::at(chrono::Utc::now());
    for path in ["members/1".to_owned(), format!("funders/{FUNDER}")] {
        let historical: serde_json::Value = serde_json::from_str(
            &client
                .get(format!("{origin}/{path}?period=historical"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
        )
        .unwrap();
        for (wire, period) in [
            ("historical", ReportingPeriod::Historical),
            ("current-term", ReportingPeriod::CurrentTerm),
            ("current-year", ReportingPeriod::CurrentYear),
        ] {
            let selected: serde_json::Value = serde_json::from_str(
                &client
                    .get(format!("{origin}/{path}?period={wire}"))
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(selected["period"], wire);
            let expected: Vec<_> = historical["declarations"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|source| {
                    let declaration = source.get("declaration").unwrap_or(source);
                    let date = declaration["registered_at"]
                        .as_str()
                        .map(|value| value.parse().unwrap());
                    context.window(period).includes(date)
                })
                .collect();
            assert_eq!(
                selected["declarations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .collect::<Vec<_>>(),
                expected
            );
            let headline = |value: &serde_json::Value| {
                value.get("summary").unwrap_or(value)["declared_totals"].clone()
            };
            assert_eq!(headline(&historical), headline(&selected));
            let entry_count: usize = expected
                .iter()
                .map(|source| {
                    source.get("declaration").unwrap_or(source)["entries"]
                        .as_array()
                        .unwrap()
                        .len()
                })
                .sum();
            assert_eq!(
                selected.get("summary").unwrap_or(&selected)["entry_count"],
                entry_count
            );
        }
        let default: serde_json::Value = serde_json::from_str(
            &client
                .get(format!("{origin}/{path}"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(default["period"], "current-term");
        for invalid in ["", "other", "Historical"] {
            assert_eq!(
                client
                    .get(format!("{origin}/{path}?period={invalid}"))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                reqwest::StatusCode::BAD_REQUEST
            );
        }
    }
    db.pool().close().await;
    let response = client
        .get(format!("{origin}/members/1?period=historical"))
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

#[sqlx::test(migrations = "../db/migrations", fixtures("details"))]
async fn preferred_profile_names_preserve_requested_identity_and_member_source_history(
    pool: PgPool,
) {
    let first: FunderId = FUNDER.parse().unwrap();
    let second = uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000003").unwrap();
    sqlx::query!(
        "INSERT INTO exposed.funders (id, funder_name) VALUES ($1, 'BBC Studios Ltd')",
        second
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, 'Exact Funder')",
        second
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency) VALUES (120, $1, 12, 'GBP')", second).execute(&pool).await.unwrap();
    let db = ExposedDatabase::from(pool);
    let before = db.member(MemberId::new(1).unwrap()).await.unwrap().unwrap();
    db.refresh_funder_groups().await.unwrap();
    for requested in [first, FunderId::new(second)] {
        let funding = db.funder_funding(requested).await.unwrap().unwrap();
        assert_eq!(funding.profile.funder.id(), requested);
        assert_eq!(funding.profile.funder.name(), "BBC");
        assert!(funding.profile.aliases.contains(&"Exact Funder".into()));
        assert!(funding.profile.aliases.contains(&"BBC Studios Ltd".into()));
        assert!(!funding.profile.aliases.contains(&"BBC".into()));
        assert_eq!(funding.profile.member_ids, [first, FunderId::new(second)]);
    }
    let source_names = |history: &crate::domain::models::entity_details::MemberHistory| {
        history
            .declarations
            .iter()
            .flat_map(|declaration| &declaration.entries)
            .filter_map(|entry| {
                entry
                    .funder
                    .as_ref()
                    .map(|funder| (funder.id, funder.name.clone()))
            })
            .collect::<Vec<_>>()
    };
    let after = db.member(MemberId::new(1).unwrap()).await.unwrap().unwrap();
    assert_eq!(source_names(&after), source_names(&before));
    assert!(source_names(&after).contains(&(FunderId::new(second), "BBC Studios Ltd".into())));
    assert!(!source_names(&after).iter().any(|(_, name)| name == "BBC"));
}

#[sqlx::test(migrations = "../db/migrations", fixtures("details"))]
async fn connected_funding_preserves_entries_requested_id_and_link_evidence(pool: PgPool) {
    let first: FunderId = FUNDER.parse().unwrap();
    let second = uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000003").unwrap();
    let third = uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000005").unwrap();
    let db = ExposedDatabase::from(pool.clone());
    let before = db.funder_funding(first).await.unwrap().unwrap();
    let before_count: usize = before
        .declarations
        .iter()
        .map(|source| source.declaration.entries.len())
        .sum();
    sqlx::query!("INSERT INTO exposed.funders (id, funder_name, funder_kind, company_number) VALUES ($1, 'Bridge Funder', 'Company', '87654321'), ($2, 'Distant Funder', NULL, NULL)", second, third).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, ' exact\tFUNDER '), ($1, 'Shared bridge'), ($2, 'SHARED BRIDGE')", second, third).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency) VALUES (120, $1, 12, 'GBP'), (120, $1, 12, 'GBP'), (200, $2, NULL, 'GBP')", second, third).execute(&pool).await.unwrap();
    db.refresh_funder_groups().await.unwrap();
    for requested in [first, FunderId::new(second), FunderId::new(third)] {
        let funding = db.funder_funding(requested).await.unwrap().unwrap();
        assert_eq!(funding.profile.funder.id(), requested);
        assert_eq!(funding.profile.funder.name(), "Exact Funder");
        assert!(funding.profile.aliases.contains(&"Bridge Funder".into()));
        assert!(funding.profile.aliases.contains(&"Distant Funder".into()));
        assert_eq!(funding.profile.company_number, None);
        assert_eq!(
            funding.profile.member_ids,
            [first, FunderId::new(second), FunderId::new(third)]
        );
        assert_eq!(
            funding
                .declarations
                .iter()
                .map(|source| source.declaration.entries.len())
                .sum::<usize>(),
            before_count + 3
        );
        let known_gbp_for_first_member =
            |declarations: &[crate::domain::models::entity_details::FunderDeclaration]| {
                declarations
                    .iter()
                    .filter(|source| source.member.id.value() == 1)
                    .flat_map(|source| &source.declaration.entries)
                    .filter(|entry| entry.currency.as_deref() == Some("GBP"))
                    .filter_map(|entry| entry.amount.as_ref())
                    .sum::<BigDecimal>()
            };
        assert_eq!(
            known_gbp_for_first_member(&funding.declarations),
            known_gbp_for_first_member(&before.declarations) + BigDecimal::from(24)
        );
        assert!(
            funding
                .profile
                .alias_links
                .iter()
                .any(|link| link.left_funder_id == FunderId::new(second)
                    && link.right_funder_id == FunderId::new(third)
                    && link.left_alias == "Shared bridge"
                    && link.right_alias == "SHARED BRIDGE"
                    && link.normalized_alias == "shared bridge")
        );
        assert!(!funding.profile.alias_links.iter().any(
            |link| link.left_funder_id == first && link.right_funder_id == FunderId::new(third)
        ));
    }
    sqlx::query!(
        "UPDATE exposed.funders SET company_number = '00123456' WHERE id = $1",
        second
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        db.funder_funding(first)
            .await
            .unwrap()
            .unwrap()
            .profile
            .company_number
            .as_deref(),
        Some("00123456")
    );
}

#[sqlx::test(migrations = "../db/migrations", fixtures("declared_totals"))]
async fn declared_totals_include_all_categories_and_preserve_coverage(pool: PgPool) {
    use crate::{
        domain::services::entity_search,
        inbound::http::{routes::routes, state::AppState},
    };
    use std::sync::Arc;
    let db = ExposedDatabase::from(pool);
    let state = AppState {
        updates_service: std::sync::Arc::new(crate::domain::services::updates::Service::new(
            db.clone(),
        )),
        rankings_service: std::sync::Arc::new(crate::domain::services::rankings::Service::new(
            db.clone(),
        )),
        entity_search_service: Arc::new(entity_search::Service::new(db.clone(), db.clone())),
        entity_details_service: Arc::new(Service::new(db)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, routes().with_state(state))
            .await
            .unwrap();
    });
    let mut expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/declared_totals.json")).unwrap();
    let client = reqwest::Client::new();
    for (id, contract) in expected.as_object_mut().unwrap() {
        let before = chrono::Datelike::year(&chrono::Utc::now());
        let response = client
            .get(format!("{origin}/members/{id}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let bytes = response.bytes().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let after = chrono::Datelike::year(&chrono::Utc::now());
        let year = body["summary"]["declared_totals"]["calendar_year"]
            .as_i64()
            .unwrap();
        assert!(year == i64::from(before) || year == i64::from(after));
        let period = match (id.as_str(), year) {
            ("7001", 2024) => (Some("300"), 2, false),
            ("7001", 2025) => (Some("25.45"), 6, true),
            ("7002" | "7007", 2025) => (Some("0"), 1, false),
            ("7004", 2025) | ("7006", 2023) => (None, 1, false),
            ("7005", 2025) | ("7007", 2024) => (None, 1, true),
            _ => (None, 0, false),
        };
        contract["declared_totals"]["calendar_year"] = serde_json::json!(year);
        contract["declared_totals"]["current_year"] = serde_json::json!({
            "amount": period.0,
            "declaration_count": period.1,
            "incomplete": period.2,
        });
        assert_eq!(
            body["summary"]["declared_totals"], contract["declared_totals"],
            "member {id}"
        );
    }
    server.abort();
}

fn assert_funder_declared_contract(body: &serde_json::Value) {
    let totals = &body["declared_totals"];
    assert_eq!(totals["current_term_start"], "2024-07-09T00:00:00Z");
    assert_eq!(
        totals["historical"],
        serde_json::json!({
            "amount": "18014398509481993.246913578", "declaration_count": 14, "incomplete": true,
        })
    );
    assert_eq!(
        totals["current_term"],
        serde_json::json!({
            "amount": "18014398509481986.246913578", "declaration_count": 1, "incomplete": true,
        })
    );
    let year = totals["calendar_year"].as_i64().unwrap();
    if year == 2026 {
        assert_eq!(totals["current_year"], totals["current_term"]);
    } else {
        assert_eq!(
            totals["current_year"],
            serde_json::json!({
                "amount": null, "declaration_count": 0, "incomplete": false,
            })
        );
    }
    assert_eq!(body["first_registered_at"], "2026-01-21T00:00:00Z");
}

fn assert_funder_declared_totals(details: &crate::domain::models::entity_details::FunderDetails) {
    assert_eq!(
        details.declared_totals.historical.amount,
        Some("18014398509481993.246913578".parse().unwrap())
    );
    assert_eq!(details.declared_totals.historical.declaration_count, 14);
    assert!(details.declared_totals.historical.incomplete);
    assert_eq!(
        details.declared_totals.current_term.amount,
        Some("18014398509481986.246913578".parse().unwrap())
    );
    assert_eq!(details.declared_totals.current_term.declaration_count, 1);
    assert!(details.declared_totals.current_term.incomplete);
    assert_eq!(
        details.first_registered_at,
        Some("2026-01-21T00:00:00Z".parse().unwrap())
    );
}
