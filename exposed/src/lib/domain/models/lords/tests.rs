use super::*;
use serde_json::{Value, json};

pub fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/lords-register-page.json"
    ))
    .unwrap()
}

pub fn capture(key: IngestionKey, date: &str, members: &[(u32, &str)]) -> CompleteLordsCapture {
    let mut page = fixture();
    let template = page["items"][0].clone();
    page["items"] = members
        .iter()
        .map(|(id, name)| {
            let mut member = template.clone();
            member["value"]["member"]["id"] = json!(id);
            member["value"]["member"]["nameDisplayAs"] = json!(name);
            member
        })
        .collect();
    page["totalResults"] = json!(members.len());
    page["take"] = json!(members.len().max(1));
    let mut builder = LordsCaptureBuilder::new(key, date.parse().unwrap());
    builder
        .push_page(serde_json::to_vec(&page).unwrap())
        .unwrap();
    builder.finish().unwrap()
}

#[test]
fn retains_every_occurrence_and_original_source_value() {
    let page = fixture();
    let bytes = serde_json::to_vec(&page).unwrap();
    let mut builder = LordsCaptureBuilder::new(
        IngestionKey::default(),
        "2026-11-12T00:00:00Z".parse().unwrap(),
    );
    builder.push_page(bytes.clone()).unwrap();
    let capture = builder.finish().unwrap();
    assert_eq!(capture.pages(), &[bytes]);
    assert_eq!(capture.members()[0].raw(), &page["items"][0]["value"]);
    let public = serde_json::to_value(capture.members()[0].register()).unwrap();
    assert_eq!(public["member"]["id"], "2175");
    assert_eq!(public["member"]["membership_end_date"], Value::Null);
    assert_eq!(public["categories"].as_array().unwrap().len(), 4);
    let roots = public["categories"][0]["interests"].as_array().unwrap();
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], roots[1]);
    assert_eq!(
        roots[0]["text"],
        "Original <script>alert(1)</script>\nline two"
    );
    assert_eq!(roots[0]["created_when"], "2012-06-14T12:49:14");
    assert_eq!(roots[0]["last_amended_when"], "2025-04-05T00:00:00.123456");
    assert_eq!(roots[0]["deleted_when"], "2025-04-06T00:00:00+01:00");
    assert_eq!(roots[0]["children"][0]["text"], Value::Null);
    assert_eq!(roots[0]["children"][0]["deleted_when"], Value::Null);
    assert_eq!(roots[0]["children"][0]["is_correction"], true);
    assert_eq!(
        roots[0]["children"][0]["children"][0]["text"],
        "Ceased but not deleted"
    );
    assert_eq!(public["categories"][1]["source_id"], 1002);
    assert_eq!(public["categories"][2]["placement"], "other");
    assert_eq!(public["categories"][3]["name"], "Nil");
}

#[test]
fn category_identity_owns_all_shareholding_versions() {
    for id in [4, 12, 31, 1002, 1003, 1004, 1005] {
        let category: LordsCategoryId = serde_json::from_value(json!(id)).unwrap();
        assert_eq!(category.placement(), LordsPlacement::Shareholdings);
    }
    for id in [1, 11, 1001, 1006, 9999] {
        let category: LordsCategoryId = serde_json::from_value(json!(id)).unwrap();
        assert_eq!(category.placement(), LordsPlacement::Other);
    }
    assert!(serde_json::from_value::<LordsCategoryId>(json!(0)).is_err());
}

#[test]
fn source_boundary_rejects_malformed_ids_dates_and_membership() {
    for (pointer, value) in [
        ("/member/id", json!(0)),
        ("/member/nameDisplayAs", json!(" ")),
        ("/member/latestHouseMembership/house", json!(1)),
        (
            "/member/latestHouseMembership/membershipEndDate",
            json!("bad"),
        ),
        ("/interestCategories/0/id", json!(-1)),
        ("/interestCategories/0/interests/0/id", json!(0)),
        (
            "/interestCategories/0/interests/0/createdWhen",
            json!("2026-02-30T00:00:00"),
        ),
        (
            "/interestCategories/0/interests/0/isCorrection",
            json!("false"),
        ),
    ] {
        let mut source = fixture()["items"][0]["value"].clone();
        *source.pointer_mut(pointer).unwrap() = value;
        assert!(
            LordRegister::from_source(&source, Utc::now()).is_err(),
            "accepted {pointer}"
        );
    }
    for value in [
        "2026-11-12T00:00:00Z",
        "2026-11-12T00:00:00-04:00",
        "2026-11-12T00:00:00",
    ] {
        assert_eq!(
            serde_json::to_value(SourceTimestamp::try_from(value.to_owned()).unwrap()).unwrap(),
            value
        );
    }
}

#[test]
fn complete_capture_requires_contiguous_stable_nonduplicated_pages() {
    let mut first = fixture();
    first["totalResults"] = json!(2);
    first["take"] = json!(1);
    let builder = || {
        let mut builder = LordsCaptureBuilder::new(IngestionKey::default(), Utc::now());
        builder
            .push_page(serde_json::to_vec(&first).unwrap())
            .unwrap();
        builder
    };
    assert!(builder().finish().is_err());
    assert!(
        builder()
            .push_page(serde_json::to_vec(&first).unwrap())
            .is_err()
    );
    let mut second = first.clone();
    second["skip"] = json!(1);
    second["items"][0]["value"]["member"]["id"] = json!(2176);
    for (pointer, value) in [
        ("/skip", json!(2)),
        ("/totalResults", json!(3)),
        ("/take", json!(0)),
        ("/items", json!([])),
        ("/items/0/value/member/id", json!(2175)),
    ] {
        let mut invalid = second.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(
            builder()
                .push_page(serde_json::to_vec(&invalid).unwrap())
                .is_err(),
            "accepted {pointer}"
        );
    }
    let mut complete = builder();
    complete
        .push_page(serde_json::to_vec(&second).unwrap())
        .unwrap();
    assert!(complete.is_complete());
    assert!(
        complete
            .push_page(serde_json::to_vec(&second).unwrap())
            .is_err()
    );
    assert_eq!(complete.finish().unwrap().members().len(), 2);
}

#[test]
fn nullable_source_collections_and_category_names_retain_their_meaning() {
    let mut source = fixture()["items"][0]["value"].clone();
    source["interestCategories"][0]["name"] = Value::Null;
    source["member"]["latestParty"] = Value::Null;
    source["member"]["latestHouseMembership"] = Value::Null;
    source["interestCategories"][0]["interests"][0]["createdWhen"] = Value::Null;
    source["interestCategories"][0]["interests"][0]["lastAmendedWhen"] = Value::Null;
    source["interestCategories"][0]["interests"][0]["childInterests"] = Value::Null;
    source["interestCategories"][1]["interests"] = Value::Null;
    let parsed = LordRegister::from_source(&source, Utc::now()).unwrap();
    assert!(parsed.categories[0].name.is_none());
    assert!(parsed.member.party_name.is_none());
    assert!(parsed.member.membership_status.is_none());
    assert!(parsed.categories[0].interests[0].created_when.is_none());
    assert!(
        parsed.categories[0].interests[0]
            .last_amended_when
            .is_none()
    );
    assert!(parsed.categories[0].interests[0].children.is_empty());
    assert!(parsed.categories[1].interests.is_empty());
    source["interestCategories"] = Value::Null;
    assert!(
        LordRegister::from_source(&source, Utc::now())
            .unwrap()
            .categories
            .is_empty()
    );
}
