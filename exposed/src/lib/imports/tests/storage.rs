use super::date;
use crate::imports::{
    adapters::postgres::PostgresStore,
    core::{
        members::{HouseMembership, Member, MemberHistory, MemberProfile},
        ports::ImportStore,
    },
};
use chrono::Utc;
use sqlx::{PgPool, Row};

fn member(id: i32) -> Member {
    Member::from_history(
        MemberProfile {
            id,
            name: format!("Member {id}"),
            party_id: Some(1),
            party_name: Some("Party".into()),
            house: 1,
            membership_from: Some("Seat".into()),
        },
        &MemberHistory {
            id,
            memberships: vec![HouseMembership {
                house: 1,
                start: date("1987-06-11"),
                end: None,
            }],
        },
        date("2024-07-04"),
        date("2026-09-15"),
        true,
    )
    .unwrap()
}

#[sqlx::test(migrations = "../db/migrations")]
async fn member_batches_preserve_ids_and_rollback_whole_failed_batch(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    let first = member(1);
    let changes = store
        .member_batch(date("2024-07-04"), &[first.clone()])
        .await
        .unwrap();
    assert_eq!(changes.inserted, 1);
    let before = sqlx::query("SELECT id, updated_at FROM exposed.members")
        .fetch_one(&pool)
        .await
        .unwrap();
    let changes = store
        .member_batch(date("2024-07-04"), &[first])
        .await
        .unwrap();
    assert_eq!(changes.unchanged, 1);
    let after = sqlx::query("SELECT id, updated_at FROM exposed.members")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        before.get::<uuid::Uuid, _>("id"),
        after.get::<uuid::Uuid, _>("id")
    );
    assert_eq!(
        before.get::<chrono::DateTime<Utc>, _>("updated_at"),
        after.get::<chrono::DateTime<Utc>, _>("updated_at")
    );
    sqlx::query("ALTER TABLE exposed.members ADD CONSTRAINT injected_failure CHECK (parliament_member_id != 3)")
        .execute(&pool).await.unwrap();
    assert!(
        store
            .member_batch(date("2024-07-04"), &[member(2), member(3)])
            .await
            .is_err()
    );
    assert_eq!(store.cohort(date("2024-07-04")).await.unwrap().len(), 1);
    assert!(
        store
            .member_batch(date("2029-07-04"), &[member(4)])
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn shared_metadata_changes_keep_unchanged_funding_ids_and_duplicate_multiplicity(
    pool: PgPool,
) {
    use crate::imports::{adapters::declarations::interpret, core::declarations::Accepted};
    let store = PostgresStore::new(pool.clone());
    store
        .member_batch(date("2024-07-04"), &[member(1)])
        .await
        .unwrap();
    let member_id = store.cohort(date("2024-07-04")).await.unwrap()[&1];
    let mut raw = super::refresh::declaration(101, 1, "10.00");
    raw["versions"][0]["fields"]
        .as_array_mut()
        .unwrap()
        .extend([
            serde_json::json!({"name":"DonorStatus","value":"Company"}),
            serde_json::json!({"name":"DonorCompanyIdentifier","value":"001234"}),
        ]);
    let mut draft = interpret(&raw).unwrap();
    draft.funding.push(draft.funding[0].clone());
    let accepted = Accepted {
        declaration: draft.clone().accept(None).unwrap(),
        fetched_at: Utc::now(),
    };
    store.publish(member_id, &[accepted.clone()]).await.unwrap();
    let before: Vec<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM exposed.funding_entries ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(before.len(), 2);
    for entry in &mut draft.funding {
        entry.company_number = Some("009999".into());
    }
    draft.funding.reverse();
    store
        .publish(
            member_id,
            &[Accepted {
                declaration: draft.clone().accept(None).unwrap(),
                fetched_at: Utc::now(),
            }],
        )
        .await
        .unwrap();
    let after: Vec<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM exposed.funding_entries ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    let company: String = sqlx::query_scalar("SELECT company_number FROM exposed.funders")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(company, "009999");
    // Omitting known metadata retains it; changing multiplicity replaces the group.
    draft.funding.truncate(1);
    draft.funding[0].funder_kind = None;
    draft.funding[0].company_number = None;
    store
        .publish(
            member_id,
            &[Accepted {
                declaration: draft.accept(None).unwrap(),
                fetched_at: Utc::now(),
            }],
        )
        .await
        .unwrap();
    let new_ids: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM exposed.funding_entries")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(new_ids.len(), 1);
    assert!(!before.contains(&new_ids[0]));
    let company: String = sqlx::query_scalar("SELECT company_number FROM exposed.funders")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(company, "009999");
}

#[sqlx::test(migrations = "../db/migrations")]
async fn declaration_database_failure_rolls_back_headers_payments_and_shared_funder_changes(
    pool: PgPool,
) {
    use crate::imports::{adapters::declarations::interpret, core::declarations::Accepted};
    let store = PostgresStore::new(pool.clone());
    store
        .member_batch(date("2024-07-04"), &[member(1)])
        .await
        .unwrap();
    let id = store.cohort(date("2024-07-04")).await.unwrap()[&1];
    let good = Accepted {
        declaration: interpret(&super::refresh::declaration(101, 1, "10"))
            .unwrap()
            .accept(None)
            .unwrap(),
        fetched_at: Utc::now(),
    };
    let bad = Accepted {
        declaration: interpret(&super::refresh::declaration(102, 1, "20"))
            .unwrap()
            .accept(None)
            .unwrap(),
        fetched_at: Utc::now(),
    };
    sqlx::query("ALTER TABLE exposed.declarations ADD CONSTRAINT injected_failure CHECK (source_declaration_id != 102)")
        .execute(&pool).await.unwrap();
    assert!(store.publish(id, &[good, bad]).await.is_err());
    for query in [
        "SELECT count(*) FROM exposed.declarations",
        "SELECT count(*) FROM exposed.funders",
        "SELECT count(*) FROM exposed.funding_entries",
    ] {
        let count: i64 = sqlx::query_scalar(query).fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0);
    }
}

#[sqlx::test(migrations = "../db/migrations")]
async fn baseline_is_reversible_and_additive_upgrade_preserves_domain_records(pool: PgPool) {
    let store = PostgresStore::new(pool.clone());
    store
        .member_batch(date("2024-07-04"), &[member(1)])
        .await
        .unwrap();
    let before: String = sqlx::query_scalar("SELECT row_to_json(m)::TEXT FROM exposed.members m")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(
        "DROP TABLE exposed.import_notifications; DROP TABLE exposed.import_refresh_state;",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../../db/upgrades/import_refresh_state.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../../db/upgrades/import_refresh_state.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let after: String = sqlx::query_scalar("SELECT row_to_json(m)::TEXT FROM exposed.members m")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    sqlx::raw_sql(include_str!(
        "../../../../../db/migrations/20260915000000_initial.down.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../../db/migrations/20260915000000_initial.up.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert!(store.cohort(date("2024-07-04")).await.unwrap().is_empty());
    assert!(
        store
            .refresh_state(date("2024-07-04"))
            .await
            .unwrap()
            .last_completed
            .is_none()
    );
}
