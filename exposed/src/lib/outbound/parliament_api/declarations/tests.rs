use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};

use super::{ApiError, decode_declaration};
use crate::domain::models::declaration_ingestion::CapturedDeclaration;

fn fetched_at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_790_985_600, 123_000_000).unwrap()
}

fn source(fields: &Value) -> Value {
    json!({
        "id": 42,
        "parentInterestId": null,
        "category": {"id": 3, "name": "Donations", "type": "Commons"},
        "registrant": {"type": "Member", "memberDetail": {"id": 4613}},
        "versions": [{
            "register": {"id": 820, "publishedDate": "2026-09-07", "type": "Commons"},
            "registrationDate": "2026-09-01",
            "fields": fields
        }]
    })
}

fn decode(source: &Value) -> Result<CapturedDeclaration, ApiError> {
    decode_declaration(source.clone(), fetched_at())
}

#[test]
fn preserves_every_version_and_the_complete_source_payload() {
    let mut source = source(&json!([]));
    source["parentInterestId"] = json!(41);
    source["unprojectedEvidence"] = json!({"notes": ["keep this", null]});
    let mut older_version = source["versions"][0].clone();
    older_version["register"]["id"] = json!(810);
    older_version["register"]["publishedDate"] = json!("2026-08-01");
    older_version["registrationDate"] = Value::Null;
    source["versions"]
        .as_array_mut()
        .unwrap()
        .push(older_version);
    let declaration = decode(&source).unwrap();

    assert_eq!(declaration.id().value(), 42);
    assert_eq!(declaration.parent_id().unwrap().value(), 41);
    assert_eq!(declaration.category_id(), 3);
    assert_eq!(declaration.category_name(), "Donations");
    assert_eq!(declaration.fetched_at(), fetched_at());
    assert_eq!(
        serde_json::from_str::<Value>(declaration.source_json()).unwrap(),
        source
    );
    let [latest, older] = declaration.versions() else {
        panic!("expected both source versions")
    };
    assert_eq!(latest.index(), 0);
    assert_eq!(latest.register_id(), 820);
    assert_eq!(
        latest.published_date(),
        NaiveDate::from_ymd_opt(2026, 9, 7).unwrap()
    );
    assert_eq!(
        latest.registration_date(),
        NaiveDate::from_ymd_opt(2026, 9, 1)
    );
    assert_eq!(older.index(), 1);
    assert_eq!(older.register_id(), 810);
    assert_eq!(
        older.published_date(),
        NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()
    );
    assert_eq!(older.registration_date(), None);
    assert_eq!(latest.groups()[0].path(), "/versions/0/fields");
    assert_eq!(older.groups()[0].path(), "/versions/1/fields");
}

#[test]
fn keeps_repeated_donor_groups_separate_without_inheriting_other_groups_fields() {
    let repeated_donor = json!([
        {"name": "Name", "value": "Alice Example"},
        {"name": "Value", "value": "120.50", "typeInfo": {"currencyCode": "EUR"}}
    ]);
    let declaration = decode(&source(&json!([
        {"name": "DonorName", "value": "Top-level donor"},
        {"name": "PaymentType", "value": "Monetary"},
        {"name": "DonorStatus", "value": "Company"},
        {"name": "DonorCompanyIdentifier", "value": "00001234"},
        {"name": "Donors", "values": [repeated_donor.clone(), repeated_donor, [
            {"name": "Name", "value": "Bob Example"},
            {"name": "Value", "value": "300.00", "typeInfo": {"currencyCode": "GBP"}},
            {"name": "PaymentType", "value": "In kind"}
        ]]}
    ])))
    .unwrap();

    let [top, first_alice, second_alice, bob] = declaration.versions()[0].groups() else {
        panic!("expected a top-level row and all three donor groups")
    };
    assert_eq!(top.donor_name(), Some("Top-level donor"));
    assert_eq!(top.amount(), None);
    assert_eq!(top.currency(), None);
    for alice in [first_alice, second_alice] {
        assert_eq!(alice.donor_name(), Some("Alice Example"));
        assert_eq!(alice.amount(), Some("120.50"));
        assert_eq!(alice.currency(), Some("EUR"));
        assert_eq!(alice.payment_type(), None);
        assert_eq!(alice.funder_kind(), None);
        assert_eq!(alice.company_number(), None);
    }
    assert_eq!(first_alice.path(), "/versions/0/fields/4/values/0");
    assert_eq!(second_alice.path(), "/versions/0/fields/4/values/1");
    assert_eq!(bob.path(), "/versions/0/fields/4/values/2");
    assert_eq!(bob.donor_name(), Some("Bob Example"));
    assert_eq!(bob.amount(), Some("300.00"));
    assert_eq!(bob.currency(), Some("GBP"));
    assert_eq!(bob.payment_type(), Some("In kind"));
}

#[test]
fn preserves_names_attribution_evidence_and_exact_numeric_amounts() {
    let amount: Value = serde_json::from_str(
        r#"{"name":"Value","value":12345678901234567890.1234500,"typeInfo":{"currencyCode":"GBP"}}"#,
    ).unwrap();
    let declaration = decode(&source(&json!([
        {"name": "UltimatePayerName", "value": " Ultimate Payer LTD. "},
        {"name": "DonorName", "value": "Donor Limited"},
        {"name": "PayerName", "value": "Payment Intermediary"},
        {"name": "IsUltimatePayerDifferent", "value": true},
        {"name": "PaymentType", "value": "In kind"},
        {"name": "DonorStatus", "value": "Other"},
        {"name": "DonorCompanyIdentifier", "value": "SC001234"},
        amount
    ])))
    .unwrap();

    let group = &declaration.versions()[0].groups()[0];
    assert_eq!(group.ultimate_payer_name(), Some(" Ultimate Payer LTD. "));
    assert_eq!(group.donor_name(), Some("Donor Limited"));
    assert_eq!(group.payer_name(), Some("Payment Intermediary"));
    assert_eq!(group.is_ultimate_payer_different(), Some(true));
    assert_eq!(group.payment_type(), Some("In kind"));
    assert_eq!(group.funder_kind(), Some("Other"));
    assert_eq!(group.company_number(), Some("SC001234"));
    assert_eq!(group.amount(), Some("12345678901234567890.1234500"));
    assert_eq!(group.currency(), Some("GBP"));
}

#[test]
fn retains_nonfinancial_declarations_and_missing_values() {
    for fields in [
        Value::Null,
        json!([]),
        json!([
            {"name": "Description", "value": "Unpaid trustee"},
            {"name": "Value", "value": null},
            {"name": "DonorName", "value": null}
        ]),
    ] {
        let declaration = decode(&source(&fields)).unwrap();
        let [group] = declaration.versions()[0].groups() else {
            panic!("expected a top-level row")
        };
        assert_eq!(group.path(), "/versions/0/fields");
        assert_eq!(group.ultimate_payer_name(), None);
        assert_eq!(group.donor_name(), None);
        assert_eq!(group.payer_name(), None);
        assert_eq!(group.amount(), None);
        assert_eq!(group.currency(), None);
        assert_eq!(group.payment_type(), None);
        assert_eq!(group.funder_kind(), None);
        assert_eq!(group.company_number(), None);
        assert_eq!(group.is_ultimate_payer_different(), None);
    }
}
