use super::*;
use crate::domain::{
    models::{lords::LordsCaptureBuilder, parliament_member::ParliamentMember},
    repositories::parliament_member_repository::ParliamentMemberRepo,
};

fn commons(id: u32, name: &str) -> ParliamentMember {
    ParliamentMember::new(
        name.into(),
        id.to_string().parse().unwrap(),
        "Commons party".into(),
        1,
        "Commons constituency".into(),
    )
}

#[sqlx::test(migrations = "../db/migrations")]
async fn shared_identity_preserves_profiles_cohort_and_duplicate_input_order(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.upsert_members(&[commons(1, "Commons first")])
        .await
        .unwrap();
    let before = sqlx::query_scalar!("SELECT to_jsonb(p) FROM exposed.commons_member_profiles p")
        .fetch_one(&pool)
        .await
        .unwrap();
    let first_uuid =
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Lord first"), (2, "Lord second"), (3, "Only Lord")],
    ))
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT to_jsonb(p) FROM exposed.commons_member_profiles p WHERE member_id = $1",
            first_uuid
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT member_id FROM exposed.lords_member_profiles WHERE member_id = $1",
            first_uuid
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        first_uuid
    );
    let second_uuid =
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id = 2")
            .fetch_one(&pool)
            .await
            .unwrap();
    let before_lord = db.lord_register("2".parse().unwrap()).await.unwrap();
    db.upsert_members(&[
        commons(2, "Earlier input"),
        commons(1, "Commons updated"),
        commons(2, "Last duplicate"),
    ])
    .await
    .unwrap();
    assert_eq!(
        db.lord_register("2".parse().unwrap()).await.unwrap(),
        before_lord
    );
    let second = sqlx::query!("SELECT member_id, name, p.created_at >= m.created_at AS clock_order FROM exposed.commons_member_profiles p JOIN exposed.members m ON m.id = p.member_id WHERE m.parliament_member_id = 2")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(second.member_id, second_uuid);
    assert_eq!(second.name, "Last duplicate");
    assert_eq!(second.clock_order, Some(true));
    sqlx::query!("UPDATE exposed.commons_member_profiles SET latest_house = 2, is_current_commons = false WHERE member_id = $1",first_uuid).execute(&pool).await.unwrap();
    assert_eq!(
        db.get_stored_member_ids()
            .await
            .unwrap()
            .iter()
            .map(|id| id.value())
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert!(db.member("1".parse().unwrap()).await.unwrap().is_some());
    assert!(db.member("3".parse().unwrap()).await.unwrap().is_none());
    assert!(sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at) SELECT 1, id, 1, 'Example', now() FROM exposed.members WHERE parliament_member_id = 3").execute(&pool).await.is_err());
}

#[sqlx::test(migrations = "../db/migrations")]
async fn concurrent_first_identity_is_shared_between_chambers(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let commons_db = db.clone();
    let commons_barrier = barrier.clone();
    let commons = tokio::spawn(async move {
        commons_barrier.wait().await;
        commons_db
            .upsert_members(&[commons(1, "Commons first")])
            .await
    });
    let lords_db = db.clone();
    let lords_barrier = barrier.clone();
    let lords = tokio::spawn(async move {
        lords_barrier.wait().await;
        lords_db
            .load_lords(&capture(
                IngestionKey::default(),
                "2026-01-01T00:00:00Z",
                &[(1, "Lords first")],
            ))
            .await
    });
    barrier.wait().await;
    commons.await.unwrap().unwrap();
    lords.await.unwrap().unwrap();
    let row = sqlx::query!("SELECT m.id, c.member_id AS commons_id, l.member_id AS lords_id FROM exposed.members m JOIN exposed.commons_member_profiles c ON c.member_id = m.id JOIN exposed.lords_member_profiles l ON l.member_id = m.id WHERE parliament_member_id = 1")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(row.id, row.commons_id);
    assert_eq!(row.id, row.lords_id);
    assert_eq!(
        db.member("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "Commons first"
    );
    assert_eq!(
        db.lord_register("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "Lords first"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn constraints_reject_invalid_occurrences_and_reads_reject_gaps(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Lord")],
    ))
    .await
    .unwrap();
    let member_id =
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    for (category, node, parent, sibling, source_id) in [
        (0, -1, None::<i64>, 20, 1),
        (0, 20, Some(-1), 20, 1),
        (0, 20, Some(20), 20, 1),
        (0, 20, Some(21), 20, 1),
        (1, 20, Some(1), 0, 1),
        (0, 20, None, -1, 1),
        (0, 20, None, 0, 1),
        (0, 20, Some(0), 0, 1),
        (0, 20, None, 20, 0),
        (0, 20, None, 20, 4_294_967_296),
        (4, 0, None, 0, 1),
    ] {
        assert!(sqlx::query!("INSERT INTO exposed.lords_interests (member_id, category_position, node_position, parent_position, sibling_position, source_id, is_correction) VALUES ($1,$2,$3,$4,$5,$6,false)",member_id,category,node,parent,sibling,source_id).execute(&pool).await.is_err(),"accepted {category}/{node}/{parent:?}/{sibling}/{source_id}");
    }
    for (category, source_id) in [(0, 1), (-1, 1), (4, 0), (4, 4_294_967_296)] {
        assert!(sqlx::query!("INSERT INTO exposed.lords_categories (member_id, category_position, source_id, sort_order) VALUES ($1,$2,$3,0)",member_id,category,source_id).execute(&pool).await.is_err());
    }
    sqlx::query!("INSERT INTO exposed.lords_categories (member_id, category_position, source_id, sort_order) VALUES ($1,5,1,0)",member_id).execute(&pool).await.unwrap();
    assert!(matches!(
        db.lord_register("1".parse().unwrap()).await,
        Err(LordsError::Database(_))
    ));
    sqlx::query!(
        "DELETE FROM exposed.lords_categories WHERE member_id = $1 AND category_position = 5",
        member_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO exposed.lords_interests (member_id, category_position, node_position, sibling_position, source_id, is_correction) VALUES ($1,3,2,1,1,false)",member_id).execute(&pool).await.unwrap();
    assert!(matches!(
        db.lord_register("1".parse().unwrap()).await,
        Err(LordsError::Database(_))
    ));
    sqlx::query!(
        "DELETE FROM exposed.lords_interests WHERE member_id = $1 AND category_position = 3 AND node_position = 2",
        member_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO exposed.lords_interests (member_id, category_position, node_position, sibling_position, source_id, is_correction) VALUES ($1,3,1,2,1,false)",member_id).execute(&pool).await.unwrap();
    assert!(matches!(
        db.lord_register("1".parse().unwrap()).await,
        Err(LordsError::Database(_))
    ));
}

#[sqlx::test(migrations = "../db/migrations")]
async fn replacement_failure_rolls_back_deleted_children_new_identities_and_ledger(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let initial = capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(2, "Initial")],
    );
    db.load_lords(&initial).await.unwrap();
    sqlx::query!("ALTER TABLE exposed.lords_interests ADD CONSTRAINT reject_replacement CHECK (interest_text IS DISTINCT FROM 'Reject replacement') NOT VALID").execute(&pool).await.unwrap();
    let mut page = fixture();
    let mut first = page["items"][0].clone();
    first["value"]["member"]["id"] = json!(1);
    let mut second = first.clone();
    second["value"]["member"]["id"] = json!(2);
    second["value"]["interestCategories"][0]["interests"][0]["interest"] =
        json!("Reject replacement");
    page["items"] = json!([first, second]);
    page["totalResults"] = json!(2);
    page["take"] = json!(2);
    let mut builder = LordsCaptureBuilder::new(
        IngestionKey::default(),
        "2026-02-01T00:00:00Z".parse().unwrap(),
    );
    builder
        .push_page(serde_json::to_vec(&page).unwrap())
        .unwrap();
    assert!(db.load_lords(&builder.finish().unwrap()).await.is_err());
    assert_eq!(
        &db.lord_register("2".parse().unwrap())
            .await
            .unwrap()
            .unwrap(),
        initial.members()[0].register()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.members WHERE parliament_member_id = 1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_imports")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn winning_empty_register_removes_previous_occurrences(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(2175, "Initial")],
    ))
    .await
    .unwrap();
    let mut page = fixture();
    page["items"][0]["value"]["interestCategories"] = json!([]);
    let mut builder = LordsCaptureBuilder::new(
        IngestionKey::default(),
        "2026-02-01T00:00:00Z".parse().unwrap(),
    );
    builder
        .push_page(serde_json::to_vec(&page).unwrap())
        .unwrap();
    let empty = builder.finish().unwrap();
    db.load_lords(&empty).await.unwrap();
    assert_eq!(
        &db.lord_register("2175".parse().unwrap())
            .await
            .unwrap()
            .unwrap(),
        empty.members()[0].register()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_interests")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_categories")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn detail_read_keeps_one_snapshot_across_profile_and_tree_queries(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let capture = capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Before")],
    );
    db.load_lords(&capture).await.unwrap();
    let mut replacement = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE exposed.lords_categories IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *replacement)
        .await
        .unwrap();
    let reader = db.clone();
    let read = tokio::spawn(async move { reader.lord_register("1".parse().unwrap()).await });
    tokio::time::timeout(std::time::Duration::from_secs(10),async {
        loop {
            let waiting = sqlx::query_scalar!("SELECT count(*) FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock' AND query LIKE 'SELECT category_position, source_id, name, sort_order FROM exposed.lords_categories%'").fetch_one(&pool).await.unwrap().unwrap();
            if waiting == 1 {break;}
            tokio::task::yield_now().await;
        }
    }).await.expect("reader reached categories after fixing its profile snapshot");
    sqlx::query!("UPDATE exposed.lords_member_profiles SET name = 'After'")
        .execute(&mut *replacement)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM exposed.lords_categories")
        .execute(&mut *replacement)
        .await
        .unwrap();
    replacement.commit().await.unwrap();
    assert_eq!(
        &read.await.unwrap().unwrap().unwrap(),
        capture.members()[0].register()
    );
    let after = db
        .lord_register("1".parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.member.name, "After");
    assert!(after.categories.is_empty());
}

#[sqlx::test(migrations = "../db/migrations")]
async fn concurrent_same_key_conflict_commits_exactly_one_complete_capture(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let key = IngestionKey::default();
    let first = capture(key.clone(), "2026-01-01T00:00:00Z", &[(1, "First")]);
    let second = capture(key, "2026-01-01T00:00:00Z", &[(1, "Second")]);
    let (left, right) = tokio::join!(db.load_lords(&first), db.load_lords(&second));
    let expected = match (left, right) {
        (Ok(LordsLoadOutcome::Loaded), Err(LordsError::Conflict)) => &first,
        (Err(LordsError::Conflict), Ok(LordsLoadOutcome::Loaded)) => &second,
        other => panic!("unexpected conflict results {other:?}"),
    };
    assert_eq!(
        &db.lord_register("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap(),
        expected.members()[0].register()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT source_document FROM exposed.lords_member_profiles")
            .fetch_one(&pool)
            .await
            .unwrap(),
        *expected.members()[0].raw()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_imports")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}
