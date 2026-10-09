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
