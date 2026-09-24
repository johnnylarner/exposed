//! Search uses shared funder identities even when they have multiple payments.

use serde_json::json;
use sqlx::PgPool;

mod common;

#[sqlx::test(
    migrations = "../db/migrations",
    fixtures(
        "../../db/fixtures/add_members.sql",
        "../../db/fixtures/add_declarations_and_funding_entries.sql"
    )
)]
async fn repeated_funding_returns_one_funder(pool: PgPool) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO exposed.funding_entries (
            source_declaration_id, funder_id, amount, currency, payment_type
         ) SELECT source_declaration_id, funder_id, amount, currency, payment_type
           FROM exposed.funding_entries CROSS JOIN generate_series(1, 4)
           WHERE source_declaration_id = 1",
    )
    .execute(&pool)
    .await?;
    let app = common::start_app(&pool).await?;
    let response = reqwest::Client::new()
        .get(format!(
            "{}/search?term=McDonald%27s&strictness=1&max_entries=10",
            app.url
        ))
        .send()
        .await?
        .error_for_status()?;
    let body: serde_json::Value = serde_json::from_slice(&response.bytes().await?)?;
    assert_eq!(
        body,
        json!({"entities": [{"name": "McDonald's", "kind": "Funder", "funder_kind": "Company"}]})
    );
    Ok(())
}
