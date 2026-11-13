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
async fn replacement_failure_rolls_back_deleted_children_new_identities_and_ledger(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let initial = capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(2, "Initial")],
    );
    db.load_lords(&initial).await.unwrap();
    sqlx::query!("ALTER TABLE exposed.simple_declarations ADD CONSTRAINT reject_replacement CHECK (declaration_text IS DISTINCT FROM 'Reject replacement') NOT VALID").execute(&pool).await.unwrap();
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
        sqlx::query_scalar!("SELECT count(*) FROM exposed.simple_declarations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT sum(jsonb_array_length(empty_categories)) FROM exposed.simple_declaration_registers")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
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
        sqlx::query_scalar!("SELECT source_document AS \"source_document!\" FROM exposed.simple_declaration_registers")
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
async fn one_profile_retains_identity_financial_history_and_zero_declaration_cohort(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.upsert_members(&[commons(1, "Original MP"), commons(2, "Zero declarations")])
        .await
        .unwrap();
    let original =
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=1")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id,member_id,category_id,category_name,fetched_at) VALUES (1,$1,1,'Employment',now())",original).execute(&pool).await.unwrap();
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Now Lord"), (2, "Zero Now Lord"), (3, "Only Lord")],
    ))
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        original
    );
    let history = db.member("1".parse().unwrap()).await.unwrap().unwrap();
    assert_eq!(history.member.name, "Now Lord");
    assert_eq!(history.member.is_current_commons, Some(false));
    assert_eq!(history.declarations.len(), 1);
    assert_eq!(
        db.get_stored_member_ids()
            .await
            .unwrap()
            .iter()
            .map(|id| id.value())
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert!(db.member("2".parse().unwrap()).await.unwrap().is_some());
    assert!(db.member("3".parse().unwrap()).await.unwrap().is_none());
    assert_eq!(
        db.lord_register("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "Now Lord"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn member_and_register_capture_clocks_are_independent(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let mut member = commons(1, "Newest profile");
    member.captured_at = Some("2026-03-01T00:00:00Z".parse().unwrap());
    member.capture_key = Some(uuid::Uuid::now_v7());
    db.upsert_members(&[member.clone()]).await.unwrap();
    let older = capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Older profile")],
    );
    db.load_lords(&older).await.unwrap();
    let register = db
        .lord_register("1".parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(register.member, member.profile);
    assert_eq!(
        register.categories,
        older.members()[0].register().categories
    );
    assert_eq!(register.captured_at, older.captured_at());
    db.upsert_members(&[commons(1, "Clock unknown legacy artifact")])
        .await
        .unwrap();
    assert_eq!(
        db.lord_register("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "Newest profile"
    );
    let newer = capture(
        IngestionKey::default(),
        "2026-04-01T00:00:00Z",
        &[(1, "Latest Lord")],
    );
    db.load_lords(&newer).await.unwrap();
    db.upsert_members(&[member]).await.unwrap();
    assert_eq!(
        db.member("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member
            .name,
        "Latest Lord"
    );
}

#[sqlx::test(migrations = "../db/migrations")]
async fn concurrent_first_capture_uses_one_identity_and_latest_profile(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let mut member = commons(1, "Newest Commons");
    member.captured_at = Some("2026-03-01T00:00:00Z".parse().unwrap());
    member.capture_key = Some(uuid::Uuid::now_v7());
    let capture = capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Earlier Lord")],
    );
    let members = [member.clone()];
    let (left, right) = tokio::join!(db.upsert_members(&members), db.load_lords(&capture));
    left.unwrap();
    right.unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM exposed.members")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
    assert_eq!(
        db.lord_register("1".parse().unwrap())
            .await
            .unwrap()
            .unwrap()
            .member,
        member.profile
    );
    assert_eq!(db.get_stored_member_ids().await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../db/migrations")]
async fn declaration_constraints_and_reconstruction_reject_invalid_occurrences(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Lord")],
    ))
    .await
    .unwrap();
    let member_id =
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=1")
            .fetch_one(&pool)
            .await
            .unwrap();
    for (node, parent, sibling, source_id) in [
        (-1, None::<i64>, 20, 1),
        (20, Some(-1), 20, 1),
        (20, Some(20), 20, 1),
        (20, Some(19), 20, 1),
        (20, None, -1, 1),
        (20, None, 0, 1),
        (20, None, 20, 0),
        (20, None, 20, 4_294_967_296),
    ] {
        assert!(sqlx::query!("INSERT INTO exposed.simple_declarations (member_id,category_position,category_source_id,category_sort_order,node_position,parent_position,sibling_position,source_id,is_correction) VALUES ($1,0,1,0,$2,$3,$4,$5,false)",member_id,node,parent,sibling,source_id).execute(&pool).await.is_err());
    }
    sqlx::query!("UPDATE exposed.simple_declarations SET category_name='Contradiction' WHERE member_id=$1 AND category_position=0 AND node_position=1",member_id).execute(&pool).await.unwrap();
    assert!(matches!(
        db.lord_register("1".parse().unwrap()).await,
        Err(LordsError::Database(_))
    ));
}

#[sqlx::test(migrations = "../db/migrations")]
async fn detail_read_keeps_one_snapshot_across_canonical_profile_and_tree(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    let capture = capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Before")],
    );
    db.load_lords(&capture).await.unwrap();
    let mut replacement = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE exposed.simple_declarations IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *replacement)
        .await
        .unwrap();
    let reader = db.clone();
    let read = tokio::spawn(async move { reader.lord_register("1".parse().unwrap()).await });
    tokio::time::timeout(std::time::Duration::from_secs(10),async {
        loop {
            let waiting=sqlx::query_scalar!("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'SELECT category_position, category_source_id, category_name, category_sort_order%'").fetch_one(&pool).await.unwrap().unwrap();
            if waiting==1 {break;}
            tokio::task::yield_now().await;
        }
    }).await.expect("reader fixed its canonical profile and register snapshot before reading declarations");
    sqlx::query!("UPDATE exposed.members SET name='After'")
        .execute(&mut *replacement)
        .await
        .unwrap();
    sqlx::query!("UPDATE exposed.simple_declaration_registers SET empty_categories='[]'::jsonb")
        .execute(&mut *replacement)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM exposed.simple_declarations")
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
async fn seeded_membership_kinds_match_every_domain_variant_and_chamber(pool: PgPool) {
    use crate::domain::models::membership::{Chamber, MembershipKind};
    use strum::IntoEnumIterator;
    let expected = MembershipKind::iter()
        .map(|kind| {
            (
                kind.to_string(),
                kind.chamber().map(|chamber| match chamber {
                    Chamber::Commons => 1_i16,
                    Chamber::Lords => 2_i16,
                }),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let stored = sqlx::query!("SELECT id,chamber_id FROM exposed.membership_kinds")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.id, row.chamber_id))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(stored, expected);
}

async fn migration_state(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar!("SELECT jsonb_build_object(
        'members',(SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM exposed.members m),
        'registers',(SELECT jsonb_agg(to_jsonb(r) ORDER BY member_id) FROM exposed.simple_declaration_registers r),
        'simple_declarations',(SELECT jsonb_agg(to_jsonb(d) ORDER BY member_id,category_position,node_position) FROM exposed.simple_declarations d),
        'declarations',(SELECT jsonb_agg(to_jsonb(d) ORDER BY source_declaration_id) FROM exposed.declarations d),
        'funding',(SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM exposed.funding_entries e),
        'imports',(SELECT jsonb_agg(to_jsonb(i) ORDER BY ingestion_key) FROM exposed.lords_imports i),
        'columns',(SELECT jsonb_agg(to_jsonb(c) ORDER BY table_name,ordinal_position) FROM information_schema.columns c WHERE table_schema='exposed'),
        'triggers',(SELECT jsonb_agg(jsonb_build_array(t.tgname,t.tgenabled) ORDER BY t.tgname) FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='exposed' AND NOT t.tgisinternal)
    )").fetch_one(pool).await.unwrap().unwrap()
}

#[sqlx::test(migrations = "../db/migrations")]
async fn downgrade_refuses_unrepresentable_members_without_changing_schema_or_records(
    pool: PgPool,
) {
    let db = ExposedDatabase::from(pool.clone());
    let mut member = commons(1, "Canonical Commons");
    member.captured_at = Some("2026-03-01T00:00:00Z".parse().unwrap());
    member.capture_key = Some(uuid::Uuid::now_v7());
    db.upsert_members(&[member]).await.unwrap();
    db.load_lords(&capture(
        IngestionKey::default(),
        "2026-01-01T00:00:00Z",
        &[(1, "Earlier Lord")],
    ))
    .await
    .unwrap();
    sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id,member_id,category_id,category_name,fetched_at) SELECT 1,id,1,'Employment',now() FROM exposed.members WHERE parliament_member_id=1").execute(&pool).await.unwrap();
    let mut migrator = sqlx::migrate!("../db/migrations");
    migrator.set_locking(false);
    migrator.dangerous_set_table_name("public._sqlx_migrations");
    for field in [
        "party_id",
        "party_name",
        "membership_from",
        "chamber",
        "cohort",
    ] {
        sqlx::query!("UPDATE exposed.members SET
            party_id=CASE WHEN $1='party_id' THEN NULL ELSE 1 END,
            party_name=CASE WHEN $1='party_name' THEN NULL ELSE 'Commons party' END,
            membership_from=CASE WHEN $1='membership_from' THEN NULL ELSE 'Commons constituency' END,
            membership_kind=CASE WHEN $1='chamber' THEN 'unknown' ELSE 'elected_mp' END,
            commons_imported_at=CASE WHEN $1='cohort' THEN NULL ELSE now() END
            WHERE parliament_member_id=1",field).execute(&pool).await.unwrap();
        let before = migration_state(&pool).await;
        let error = migrator.undo(&pool, 20_261_012_221_443).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Cannot revert member membership migration"),
            "{field}: {error}"
        );
        assert_eq!(migration_state(&pool).await, before, "{field}");
    }
}

#[sqlx::test(migrations = "../db/migrations")]
async fn representable_canonical_updates_survive_down_and_up(pool: PgPool) {
    let db = ExposedDatabase::from(pool.clone());
    db.upsert_members(&[commons(1, "Updated canonical member")])
        .await
        .unwrap();
    sqlx::query!("UPDATE exposed.members SET membership_kind='life_peer',membership_from='Life peer',party_name='Updated party' WHERE parliament_member_id=1").execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO exposed.declarations (source_declaration_id,member_id,category_id,category_name,fetched_at) SELECT 1,id,1,'Employment',now() FROM exposed.members WHERE parliament_member_id=1").execute(&pool).await.unwrap();
    let before = db.member("1".parse().unwrap()).await.unwrap().unwrap();
    let id = sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut migrator = sqlx::migrate!("../db/migrations");
    migrator.set_locking(false);
    migrator.dangerous_set_table_name("public._sqlx_migrations");
    migrator.undo(&pool, 20_261_012_221_443).await.unwrap();
    migrator.run(&pool).await.unwrap();
    let after = db.member("1".parse().unwrap()).await.unwrap().unwrap();
    assert_eq!(after.member.name, before.member.name);
    assert_eq!(after.member.party_name, before.member.party_name);
    assert_eq!(after.member.membership_from, before.member.membership_from);
    assert_eq!(after.member.is_current_commons, Some(false));
    assert_eq!(after.declarations.len(), before.declarations.len());
    assert_eq!(
        sqlx::query_scalar!("SELECT id FROM exposed.members WHERE parliament_member_id=1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        id
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT membership_kind FROM exposed.members WHERE parliament_member_id=1"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "life_peer"
    );
}
