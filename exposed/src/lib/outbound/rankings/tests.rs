use crate::{
    domain::{
        models::funder::FunderId,
        services::{
            entity_details::{EntityDetailsService, Service as DetailsService},
            rankings::{RankingsService, Service},
        },
    },
    outbound::ExposedDatabase,
};
use sqlx::PgPool;

#[sqlx::test(migrations = "../db/migrations", fixtures("rankings"))]
async fn rankings_preserve_summary_parity_canonical_identity_and_party_attribution(pool: PgPool) {
    let db = ExposedDatabase::from(pool);
    let rankings = Service::new(db.clone()).rankings().await.unwrap();
    let details = DetailsService::new(db.clone());
    assert_eq!(rankings.members.len(), 6);
    for row in &rankings.members {
        let member = details
            .member(row.identity.id.value().to_string().parse().unwrap())
            .await
            .unwrap();
        let expected = member.summary.declared_totals.current_term;
        assert_eq!(row.total.amount, expected.amount);
        assert_eq!(row.total.declaration_count, expected.declaration_count);
        assert_eq!(row.total.incomplete, expected.incomplete);
    }
    let first = &rankings.members[0];
    assert_eq!(first.identity.id.value(), 1);
    assert_eq!(
        first.total.amount,
        Some("52.56913578246913578".parse().unwrap())
    );
    assert_eq!(first.total.declaration_count, 2);
    assert!(first.total.incomplete);
    assert_eq!(rankings.members[2].identity.id.value(), 6);
    assert_eq!(
        rankings.members[2].total.amount,
        Some("-1".parse().unwrap())
    );
    assert_eq!(rankings.members[3].identity.id.value(), 3);
    assert_eq!(rankings.members[3].total.amount, None);
    assert!(rankings.members[3].total.incomplete);
    assert_eq!(rankings.members[4].identity.id.value(), 4);
    assert_eq!(rankings.members[4].total.declaration_count, 1);
    assert!(!rankings.members[4].total.incomplete);
    assert_eq!(rankings.members[5].identity.id.value(), 5);
    assert!(!rankings.members[5].total.incomplete);
    assert_eq!(rankings.companies.len(), 1);
    let company = &rankings.companies[0];
    assert_eq!(
        company.identity.id,
        "10000000-0000-0000-0000-000000000001"
            .parse::<FunderId>()
            .unwrap()
    );
    assert_eq!(company.identity.name, "Preferred company");
    assert_eq!(
        company.total.amount,
        Some("2.56913578246913578".parse().unwrap())
    );
    assert_eq!(company.total.declaration_count, 2);
    assert!(company.total.incomplete);
    assert_eq!(rankings.individuals.len(), 2);
    assert_eq!(rankings.individuals[0].identity.name, "Individual");
    assert_eq!(
        rankings.individuals[0].total.amount,
        Some("0".parse().unwrap())
    );
    assert_eq!(rankings.individuals[0].total.declaration_count, 2);
    assert!(!rankings.individuals[0].total.incomplete);
    assert_eq!(
        rankings.individuals[1].total.amount,
        Some("-2".parse().unwrap())
    );
    assert_eq!(rankings.parties.len(), 2);
    assert_eq!(rankings.parties[0].identity.id, 2);
    assert_eq!(rankings.parties[0].identity.name, "Latest Party");
    assert_eq!(
        rankings.parties[0].total.amount,
        Some("10".parse().unwrap())
    );
    assert_eq!(rankings.parties[0].total.declaration_count, 2);
    assert_eq!(
        rankings.parties[1].total.amount,
        Some("1.56913578246913578".parse().unwrap())
    );
    assert_eq!(rankings.parties[1].total.declaration_count, 5);
    assert!(rankings.parties[1].total.incomplete);
}

#[sqlx::test(migrations = "../db/migrations", fixtures("rankings"))]
async fn http_rankings_expose_four_bounded_boards_with_exact_wire_totals(pool: PgPool) {
    use crate::{
        domain::services::entity_search,
        inbound::http::{routes::routes, state::AppState},
    };
    use std::sync::Arc;
    let db = ExposedDatabase::from(pool);
    let state = AppState {
        entity_search_service: Arc::new(entity_search::Service::new(db.clone(), db.clone())),
        entity_details_service: Arc::new(DetailsService::new(db.clone())),
        rankings_service: Arc::new(Service::new(db)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, routes().with_state(state))
            .await
            .unwrap();
    });
    let response = reqwest::get(format!("{origin}/rankings")).await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(body["current_term_start"], "2024-07-09T00:00:00Z");
    assert_eq!(body["members"][0]["id"], "1");
    assert_eq!(body["members"][0]["context"], "Party A · Constituency 1");
    assert_eq!(
        body["members"][0]["total"]["amount"],
        "52.56913578246913578"
    );
    assert_eq!(body["members"][0]["total"]["declaration_count"], 2);
    assert_eq!(body["members"][0]["total"]["incomplete"], true);
    assert_eq!(body["individuals"][0]["total"]["amount"], "0");
    assert_eq!(
        body["companies"][0]["id"],
        "10000000-0000-0000-0000-000000000001"
    );
    assert!(body["parties"][0].get("context").is_none());
    for board in ["members", "individuals", "companies", "parties"] {
        assert!(body[board].as_array().unwrap().len() <= 10);
    }
    server.abort();
}
