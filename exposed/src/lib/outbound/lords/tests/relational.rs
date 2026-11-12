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
    let before = sqlx::query_scalar!(
        "SELECT to_jsonb(p) FROM exposed.member_profiles p WHERE p.chamber = 'commons'"
    )
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
            "SELECT to_jsonb(p) FROM exposed.member_profiles p WHERE p.chamber = 'commons' AND member_id = $1",
            first_uuid
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT member_id FROM exposed.member_profiles WHERE chamber = 'lords' AND member_id = $1",
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
    let second = sqlx::query!("SELECT member_id, name, p.created_at >= m.created_at AS clock_order FROM exposed.member_profiles p JOIN exposed.members m ON m.id = p.member_id WHERE p.chamber = 'commons' AND m.parliament_member_id = 2")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(second.member_id, second_uuid);
    assert_eq!(second.name, "Last duplicate");
    assert_eq!(second.clock_order, Some(true));
    sqlx::query!("UPDATE exposed.member_profiles SET latest_house = 2, is_current_commons = false WHERE chamber = 'commons' AND member_id = $1",first_uuid).execute(&pool).await.unwrap();
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
    let row = sqlx::query!("SELECT m.id, c.member_id AS commons_id, l.member_id AS lords_id FROM exposed.members m JOIN exposed.member_profiles c ON c.member_id = m.id AND c.chamber = 'commons' JOIN exposed.member_profiles l ON l.member_id = m.id AND l.chamber = 'lords' WHERE parliament_member_id = 1")
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
    sqlx::query!("UPDATE exposed.member_profiles SET name = 'After' WHERE chamber = 'lords'")
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
        sqlx::query_scalar!("SELECT source_document AS \"source_document!\" FROM exposed.member_profiles WHERE chamber = 'lords'")
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

#[sqlx::test(migrations = "../db/migrations")]
async fn profiles_reject_missing_required_fields_and_other_chamber_fields(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.upsert_members(&[commons(1, "Commons")]).await.unwrap();
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Lord")],
    ))
    .await
    .unwrap();
    let target = sqlx::query_scalar!(
        "INSERT INTO exposed.members (parliament_member_id) VALUES (2) RETURNING id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    for (field, value) in [
        ("party_id", Value::Null),
        ("party_name", Value::Null),
        ("latest_house", Value::Null),
        ("latest_membership_from", Value::Null),
        ("is_current_commons", Value::Null),
        ("created_at", Value::Null),
        ("updated_at", Value::Null),
        ("membership_from", json!("Lords")),
        ("membership_status", json!("Active")),
        ("membership_end_date", json!("2026-01-01")),
        ("source_document", json!({})),
        ("captured_at", json!("2026-01-01T00:00:00Z")),
        ("ingestion_key", json!(uuid::Uuid::nil())),
    ] {
        let patch = json!({"member_id": target, field: value});
        let error = sqlx::query!(
            "INSERT INTO exposed.member_profiles
             SELECT candidate.* FROM exposed.member_profiles p
             CROSS JOIN LATERAL jsonb_populate_record(NULL::exposed.member_profiles, to_jsonb(p) || $1) candidate
             WHERE p.chamber = 'commons'", patch
        ).execute(&pool).await.unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("23514"),
            "{field}"
        );
    }
    for (field, value) in [
        ("source_document", Value::Null),
        ("captured_at", Value::Null),
        ("ingestion_key", Value::Null),
        ("party_id", json!(1)),
        ("latest_house", json!(2)),
        ("latest_membership_from", json!("Commons")),
        ("is_current_commons", json!(false)),
        ("created_at", json!("2026-01-01T00:00:00Z")),
        ("updated_at", json!("2026-01-01T00:00:00Z")),
    ] {
        let patch = json!({"member_id": target, field: value});
        let error = sqlx::query!(
            "INSERT INTO exposed.member_profiles
             SELECT candidate.* FROM exposed.member_profiles p
             CROSS JOIN LATERAL jsonb_populate_record(NULL::exposed.member_profiles, to_jsonb(p) || $1) candidate
             WHERE p.chamber = 'lords'", patch
        ).execute(&pool).await.unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("23514"),
            "{field}"
        );
    }
    sqlx::query!("UPDATE exposed.member_profiles SET party_name = NULL, membership_from = NULL, membership_status = NULL, membership_end_date = NULL WHERE chamber = 'lords'")
        .execute(&pool).await.unwrap();
    let lord = db
        .lord_register("1".parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lord.member.party_name, None);
    assert_eq!(lord.member.membership_from, None);
    assert_eq!(lord.member.membership_status, None);
    assert_eq!(lord.member.membership_end_date, None);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn child_ownership_and_profile_deletion_preserve_the_other_chamber(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.upsert_members(&[commons(1, "Dual"), commons(2, "Commons only")])
        .await
        .unwrap();
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Dual Lord"), (3, "Lords only")],
    ))
    .await
    .unwrap();
    let error = sqlx::query!("INSERT INTO exposed.lords_categories (member_id, category_position, source_id, sort_order) SELECT id, 0, 1, 0 FROM exposed.members WHERE parliament_member_id = 2")
        .execute(&pool).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23503")
    );
    let error = sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at) SELECT 1, id, 1, 'Example', now() FROM exposed.members WHERE parliament_member_id = 3")
        .execute(&pool).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23503")
    );
    sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at) SELECT 1, id, 1, 'Example', now() FROM exposed.members WHERE parliament_member_id = 1")
        .execute(&pool).await.unwrap();
    let before = sqlx::query_scalar!("SELECT to_jsonb(p) FROM exposed.member_profiles p JOIN exposed.members m ON m.id = p.member_id WHERE m.parliament_member_id = 1 AND p.chamber = 'commons'")
        .fetch_one(&pool).await.unwrap();
    let error = sqlx::query!("DELETE FROM exposed.member_profiles p USING exposed.members m WHERE p.member_id = m.id AND m.parliament_member_id = 1 AND p.chamber = 'commons'")
        .execute(&pool).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23503")
    );
    sqlx::query!("DELETE FROM exposed.member_profiles p USING exposed.members m WHERE p.member_id = m.id AND m.parliament_member_id = 1 AND p.chamber = 'lords'")
        .execute(&pool).await.unwrap();
    assert_eq!(sqlx::query_scalar!("SELECT to_jsonb(p) FROM exposed.member_profiles p JOIN exposed.members m ON m.id = p.member_id WHERE m.parliament_member_id = 1 AND p.chamber = 'commons'")
        .fetch_one(&pool).await.unwrap(), before);
    let member = db.member("1".parse().unwrap()).await.unwrap().unwrap();
    assert_eq!(member.member.name, "Dual");
    assert_eq!(member.declarations.len(), 1);
    assert_eq!(member.declarations[0].source_id, 1);
    assert!(
        db.lord_register("1".parse().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_categories c JOIN exposed.members m ON m.id = c.member_id WHERE m.parliament_member_id = 1").fetch_one(&pool).await.unwrap(),Some(0));
    assert_eq!(sqlx::query_scalar!("SELECT count(*) FROM exposed.lords_interests i JOIN exposed.members m ON m.id = i.member_id WHERE m.parliament_member_id = 1").fetch_one(&pool).await.unwrap(),Some(0));
    db.upsert_members(&[commons(3, "Temporary Commons")])
        .await
        .unwrap();
    let lord_before = db.lord_register("3".parse().unwrap()).await.unwrap();
    sqlx::query!("DELETE FROM exposed.member_profiles p USING exposed.members m WHERE p.member_id = m.id AND m.parliament_member_id = 3 AND p.chamber = 'commons'")
        .execute(&pool).await.unwrap();
    assert!(db.member("3".parse().unwrap()).await.unwrap().is_none());
    assert_eq!(
        db.lord_register("3".parse().unwrap()).await.unwrap(),
        lord_before
    );
}
