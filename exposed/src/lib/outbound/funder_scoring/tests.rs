use super::*;
use crate::domain::models::declaration_resolution::ResolutionInput;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Reference {
    cases: Vec<ReferenceCase>,
}

#[derive(Deserialize)]
struct ReferenceCase {
    name: String,
    rows: Vec<ComparisonRow>,
    pairs: Vec<ScoredPair>,
}

fn reference() -> Reference {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/funder-resolution.json"
    )))
    .unwrap()
}

#[test]
fn native_duckdb_matches_original_splink_candidates_levels_and_probabilities() {
    let model = FrozenModel::bundled().unwrap();
    for mut case in reference().cases {
        for _ in 0..2 {
            let (actual, version) = model
                .execute(&case.rows, case.pairs.len().max(1))
                .unwrap_or_else(|error| panic!("{}: {error}", case.name));
            assert_eq!(version, "1.4.4");
            assert_eq!(actual.len(), case.pairs.len(), "{}", case.name);
            for (actual, expected) in actual.iter().zip(&case.pairs) {
                assert_eq!(actual.left, expected.left, "{}", case.name);
                assert_eq!(actual.right, expected.right, "{}", case.name);
                assert_eq!(actual.name_level, expected.name_level, "{}", case.name);
                assert_eq!(
                    actual.address_level, expected.address_level,
                    "{}",
                    case.name
                );
                assert!(
                    (actual.probability - expected.probability).abs()
                        <= expected.probability.abs() * 1e-12,
                    "{} pair {} / {}: {} != {}",
                    case.name,
                    actual.left,
                    actual.right,
                    actual.probability,
                    expected.probability
                );
            }
            case.rows.reverse();
        }
    }
}

#[test]
fn candidate_budget_refuses_the_complete_count_instead_of_truncating() {
    let model = FrozenModel::bundled().unwrap();
    let case = reference()
        .cases
        .into_iter()
        .find(|case| case.name == "overlapping_rules")
        .unwrap();
    let error = model.execute(&case.rows, 2).err().unwrap().to_string();
    assert!(error.contains("candidate budget 2 exceeded by 3 profile pairs"));
    assert_eq!(model.execute(&case.rows, 3).unwrap().0.len(), 3);
}

#[tokio::test]
async fn scorer_returns_checked_empty_scores_with_actual_native_provenance() {
    let input = ResolutionInput::new(vec![], vec![], BTreeMap::new()).unwrap();
    let input = input.scoring_input(10);
    let scorer = DuckDbFunderScorer::new().unwrap();
    let (first, second) = tokio::join!(scorer.score(&input), scorer.score(&input));
    let result = first.unwrap();
    assert!(result.pairs.is_empty());
    assert_eq!(result.model, second.unwrap().model);
    assert_eq!(result.model["backend"], "duckdb");
    assert_eq!(result.model["duckdb_version"], "1.4.4");
    assert_eq!(result.model["generator"]["splink_version"], "4.0.17");
    assert_eq!(result.model["model_version"], "funder-frozen-v2");
    assert_eq!(result.model["calibrated"], false);
    assert_eq!(result.model["sql_sha256"].as_str().unwrap().len(), 64);
    assert!(result.model.get("python_version").is_none());
    assert!(result.model.get("splink_version").is_none());
}
