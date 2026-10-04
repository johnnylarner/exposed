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
fn keeps_only_the_latest_funding_and_its_source_identifiers() {
    let mut source = source(&json!([
        {"name": "DonorName", "value": "Current donor"},
        {"name": "Value", "value": "200.00"}
    ]));
    source["parentInterestId"] = json!(41);
    source["unprojectedEvidence"] = json!({"notes": ["keep this", null]});
    let latest = source["versions"][0].clone();
    let mut older = latest.clone();
    older["register"]["id"] = json!(999);
    older["register"]["publishedDate"] = json!("2026-08-01");
    older["fields"] = json!([
        {"name": "DonorName", "value": "Old donor"},
        {"name": "Value", "value": "100.00"}
    ]);

    for versions in [json!([older, latest]), json!([latest, older])] {
        source["versions"] = versions;
        let declaration = decode(&source).unwrap();

        assert_eq!(declaration.id().value(), 42);
        assert_eq!(declaration.parent_id().unwrap().value(), 41);
        assert_eq!(declaration.category_id(), 3);
        assert_eq!(declaration.category_name(), "Donations");
        assert_eq!(declaration.fetched_at(), fetched_at());
        assert_eq!(declaration.register_id(), 820);
        assert_eq!(
            declaration.register_published_date(),
            NaiveDate::from_ymd_opt(2026, 9, 7).unwrap()
        );
        assert_eq!(
            declaration.registration_date(),
            NaiveDate::from_ymd_opt(2026, 9, 1)
        );
        let [funding] = declaration.funding_entries() else {
            panic!("expected only the latest funding entry")
        };
        assert_eq!(funding.donor_name(), Some("Current donor"));
        assert_eq!(funding.amount(), Some("200.00"));
        assert_eq!(
            serde_json::from_str::<Value>(declaration.source_json()).unwrap(),
            source
        );
    }
}

#[test]
fn keeps_multiple_funding_entries_with_their_own_details() {
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

    let [top, first_alice, second_alice, bob] = declaration.funding_entries() else {
        panic!("expected the main funding entry and all three donors")
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

    let group = &declaration.funding_entries()[0];
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
fn retains_nonfinancial_declarations_with_no_funding_entries() {
    for fields in [
        Value::Null,
        json!([]),
        json!([
            {"name": "Description", "value": "Unpaid trustee"}
        ]),
    ] {
        let declaration = decode(&source(&fields)).unwrap();
        assert_eq!(declaration.id().value(), 42);
        assert!(declaration.funding_entries().is_empty());
    }
}

#[test]
fn donor_lists_do_not_add_an_empty_funding_entry() {
    let declaration = decode(&source(&json!([
        {"name": "Purpose", "value": "Conference visit"},
        {"name": "Donors", "values": [[
            {"name": "Name", "value": "Travel sponsor"},
            {"name": "Value", "value": null}
        ]]}
    ])))
    .unwrap();
    let [entry] = declaration.funding_entries() else {
        panic!("expected one donor funding entry")
    };
    assert_eq!(entry.donor_name(), Some("Travel sponsor"));
    assert_eq!(entry.amount(), None);
    assert_eq!(entry.currency(), None);
    assert_eq!(entry.payment_type(), None);
}
