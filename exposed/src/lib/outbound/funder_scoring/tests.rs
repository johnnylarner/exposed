use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    rows: Vec<ComparisonRow>,
    pairs: Vec<ScoredPair>,
}
#[test]
fn original_splink_golden_candidates_levels_and_probabilities() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/funder-resolution.json"
    ))
    .unwrap();
    assert_eq!(fixture.cases.len(), 43);
    let scorer = WeldrsFunderScorer::new().unwrap();
    for case in fixture.cases {
        let rows = case.rows;
        for reverse in [false, true] {
            let mut input = rows.clone();
            if reverse {
                input.reverse();
            }
            let actual = infer(input, case.pairs.len(), &scorer.model)
                .unwrap_or_else(|e| panic!("{}: {e}", case.name));
            assert_eq!(actual.len(), case.pairs.len(), "{}", case.name);
            for (actual, expected) in actual.iter().zip(&case.pairs) {
                assert_eq!(
                    (&actual.left, &actual.right),
                    (&expected.left, &expected.right),
                    "{}",
                    case.name
                );
                assert_eq!(
                    (actual.name_level, actual.address_level),
                    (expected.name_level, expected.address_level),
                    "{}",
                    case.name
                );
                assert!(
                    (actual.probability - expected.probability).abs()
                        <= 1e-12 * expected.probability.abs().max(1e-12),
                    "{}: {} vs {}",
                    case.name,
                    actual.probability,
                    expected.probability
                );
            }
        }
    }
}

#[test]
fn individual_donor_shared_key_generates_native_candidate_without_name_or_address_blocks() {
    let rows = ["lord michael farmer", "michael farmer"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| ComparisonRow {
            key: format!("profile-{index}"),
            name: name.into(),
            address: None,
            blocking_keys: vec!["individual-donor-name:michael farmer".into()],
            needs_resolution: true,
        })
        .collect::<Vec<_>>();
    assert!(
        blocking::bounded_candidates(&rows, &blocking::CandidateRule::ALL[..4], 10)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        blocking::bounded_candidates(&rows, &[blocking::CandidateRule::SharedBlockingKey], 10)
            .unwrap()
            .len(),
        1
    );
    let scorer = WeldrsFunderScorer::new().unwrap();
    assert_eq!(infer(rows, 10, &scorer.model).unwrap().len(), 1);
}

#[test]
fn peerage_context_generates_native_candidates_from_checked_observations() {
    use crate::domain::models::declaration_resolution::{Observation, ResolutionInput};
    let names = [
        "Baron Vale of Mere (Alice Vale)",
        "Lord Vale of Mere",
        "Alice Vale",
    ];
    let observations = names.iter().enumerate().map(|(index, name)| {
        let id = index + 1;
        serde_json::from_value::<Observation>(serde_json::json!({
            "funder_id": format!("1/{id}/1/declaration/donor"),
            "parliament_member_id": 1, "declaration_id": id, "register_id": 1,
            "role": "donor", "source_scope": "declaration", "source_pointer": "/versions/0/fields",
            "funding_entry_id": null, "funding_ordinal": null, "donor_kind": "Individual",
            "donor_company_number": null, "name_raw": name, "name_normalized": name,
            "person_name_normalized": null, "organisation_core": null,
            "alias_normalized": [], "address_normalized": null, "address_match_quality": "unavailable"
        })).unwrap()
    }).collect();
    let input =
        ResolutionInput::new(observations, Vec::new(), std::collections::BTreeMap::new()).unwrap();
    let rows = input.scoring_input(10).rows;
    assert!(
        blocking::bounded_candidates(&rows, &blocking::CandidateRule::ALL[..4], 10)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        blocking::bounded_candidates(&rows, &[blocking::CandidateRule::SharedBlockingKey], 10)
            .unwrap()
            .len(),
        3
    );
    let scorer = WeldrsFunderScorer::new().unwrap();
    let pairs = infer(rows, 10, &scorer.model).unwrap();
    assert_eq!(pairs.len(), 3);
    assert!(pairs.iter().all(|pair| pair.probability < 0.999));
}
