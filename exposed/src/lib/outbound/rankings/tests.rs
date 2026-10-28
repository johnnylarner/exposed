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
    let rankings = Service::new(db.clone())
        .rankings(crate::domain::models::reporting_period::ReportingPeriod::CurrentTerm)
        .await
        .unwrap();
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
    assert_eq!(rankings.companies.len(), 3);
    assert_eq!(rankings.companies[0].identity.name, "Trust root");
    assert_eq!(
        rankings.companies[0].total.amount,
        Some("8".parse().unwrap())
    );
    assert_eq!(rankings.companies[1].identity.name, "Unclassified funder");
    assert_eq!(
        rankings.companies[1].identity.kind,
        crate::domain::models::funder::FunderKind::NotSpecified
    );
    assert_eq!(
        rankings.companies[1].total.amount,
        Some("4".parse().unwrap())
    );
    let company = &rankings.companies[2];
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
async fn http_rankings_expose_four_complete_boards_with_exact_wire_totals(pool: PgPool) {
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
        "10000000-0000-0000-0000-000000000004"
    );
    assert_eq!(
        body["companies"][1]["id"],
        "10000000-0000-0000-0000-000000000007"
    );
    assert_eq!(
        body["companies"][2]["id"],
        "10000000-0000-0000-0000-000000000001"
    );
    assert!(body["parties"][0].get("context").is_none());

    assert_eq!(body["period"], "current-term");
    for period in ["historical", "current-term", "current-year"] {
        let response = reqwest::get(format!("{origin}/rankings?period={period}"))
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let selected: serde_json::Value =
            serde_json::from_str(&response.text().await.unwrap()).unwrap();
        assert_eq!(selected["period"], period);
        if period == "historical" {
            assert_eq!(selected["members"][0]["total"]["declaration_count"], 4);
            assert_eq!(
                selected["companies"][0]["total"]["amount"],
                "2000002.56913578246913578"
            );
        }
    }
    for invalid in ["", "other", "Historical"] {
        assert_eq!(
            reqwest::get(format!("{origin}/rankings?period={invalid}"))
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::BAD_REQUEST
        );
    }
    server.abort();
}

#[sqlx::test(migrations = "../db/migrations", fixtures("rankings"))]
async fn windows_filter_database_rows_before_ranking_and_keep_distinct_board_scopes(pool: PgPool) {
    use crate::domain::{
        models::{
            entity_details::DeclarationCategory,
            reporting_period::{ReportingContext, ReportingPeriod},
        },
        repositories::rankings::RankingsRepo,
    };
    let context = ReportingContext::at("2025-06-01T00:00:00Z".parse().unwrap());
    let db = ExposedDatabase::from(pool.clone());
    let mut baseline = Vec::new();
    for period in [
        ReportingPeriod::Historical,
        ReportingPeriod::CurrentTerm,
        ReportingPeriod::CurrentYear,
    ] {
        let rows = db
            .ranking_declarations(context.window(period))
            .await
            .unwrap();
        let boards = crate::domain::services::rankings::summarize(rows, period, context);
        let companies: Vec<_> = boards
            .companies
            .into_iter()
            .map(|row| {
                (
                    row.identity.id,
                    row.total.amount,
                    row.total.declaration_count,
                    row.total.incomplete,
                )
            })
            .collect();
        baseline.push((period, companies));
    }
    sqlx::query!("INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons)
        SELECT ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, i, 'Scope ' || i, 1, 'Party A', 1, 'Place', true FROM generate_series(20, 32) i").execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date)
        SELECT i * 100, ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, 987, 'Other', now(), '2024-08-01'::timestamptz FROM generate_series(20, 30) i
        UNION ALL SELECT 3100, '00000000-0000-0000-0000-000000000031'::uuid, 987, 'Historical leader', now(), '2023-01-01'::timestamptz
        UNION ALL SELECT 3200, '00000000-0000-0000-0000-000000000032'::uuid, 987, 'Year leader', now(), '2025-12-31T23:59:59.999999Z'::timestamptz
        UNION ALL SELECT 3201, '00000000-0000-0000-0000-000000000032'::uuid, 987, 'Older negative', now(), '2024-12-31T23:59:59.999999Z'::timestamptz
        UNION ALL SELECT 3202, '00000000-0000-0000-0000-000000000032'::uuid, 987, 'Next year', now(), '2026-01-01'::timestamptz").execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO exposed.funding_entries (source_declaration_id, amount, currency)
        SELECT i * 100, 1000, 'GBP' FROM generate_series(20, 30) i
        UNION ALL SELECT 3100, 10000000, 'GBP'
        UNION ALL SELECT 3200, 50000, 'GBP'
        UNION ALL SELECT 3201, -100000, 'GBP'
        UNION ALL SELECT 3202, 200, 'GBP'"
    )
    .execute(&pool)
    .await
    .unwrap();
    let db = ExposedDatabase::from(pool);
    for (period, expected_leader, contains_undated) in [
        (ReportingPeriod::Historical, 31, true),
        (ReportingPeriod::CurrentTerm, 20, false),
        (ReportingPeriod::CurrentYear, 32, false),
    ] {
        let sources = db
            .ranking_declarations(context.window(period))
            .await
            .unwrap();
        assert_eq!(
            sources
                .iter()
                .any(|source| source.declaration.registered_at.is_none()),
            contains_undated
        );
        if period == ReportingPeriod::CurrentYear {
            assert!(
                sources
                    .iter()
                    .any(|source| source.declaration.source_id == 3200)
            );
            assert!(
                !sources
                    .iter()
                    .any(|source| source.declaration.source_id == 3202)
            );
        }
        let rankings = crate::domain::services::rankings::summarize(sources, period, context);
        assert_eq!(rankings.members[0].identity.id.value(), expected_leader);
        assert_eq!(
            rankings.members.len(),
            if period == ReportingPeriod::CurrentYear {
                7
            } else if period == ReportingPeriod::Historical {
                19
            } else {
                18
            }
        );
        let companies: Vec<_> = rankings
            .companies
            .into_iter()
            .map(|row| {
                (
                    row.identity.id,
                    row.total.amount,
                    row.total.declaration_count,
                    row.total.incomplete,
                )
            })
            .collect();
        assert_eq!(
            companies,
            baseline
                .iter()
                .find(|(scope, _)| *scope == period)
                .unwrap()
                .1
        );
        let all = db
            .ranking_declarations(context.window(period))
            .await
            .unwrap();
        assert!(
            all.iter()
                .filter(|s| s.declaration.category == DeclarationCategory::Other(987))
                .all(|s| s.declaration.category.source_id() == 987)
        );
    }
}

#[sqlx::test(
    migrations = "../db/migrations",
    fixtures("rankings", "rankings_complete")
)]
async fn snapshot_retains_every_identity_in_all_four_ordered_boards(pool: PgPool) {
    use crate::domain::models::reporting_period::ReportingPeriod;
    let rankings = Service::new(ExposedDatabase::from(pool))
        .rankings(ReportingPeriod::CurrentTerm)
        .await
        .unwrap();
    assert_eq!(rankings.members.len(), 29);
    assert_eq!(rankings.individuals.len(), 25);
    assert_eq!(rankings.companies.len(), 26);
    assert_eq!(rankings.parties.len(), 25);
    assert_eq!(
        rankings
            .members
            .iter()
            .take(23)
            .map(|row| row.identity.id.value())
            .collect::<Vec<_>>(),
        (100..=122).collect::<Vec<_>>()
    );
    assert_eq!(
        rankings
            .parties
            .iter()
            .take(23)
            .map(|row| row.identity.id)
            .collect::<Vec<_>>(),
        (100..=122).collect::<Vec<_>>()
    );
    for board in [&rankings.individuals, &rankings.companies] {
        assert_eq!(
            board
                .iter()
                .take(23)
                .map(|row| row.total.amount.clone())
                .collect::<Vec<_>>(),
            (100..=122)
                .map(|id| Some((1000 - id).to_string().parse().unwrap()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            board
                .iter()
                .take(23)
                .map(|row| row.identity.id.value())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            23
        );
    }
}
