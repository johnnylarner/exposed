use super::core::members::{HouseMembership, MemberHistory};
use chrono::NaiveDate;

fn date(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

#[test]
fn service_clips_continuous_membership_but_preserves_a_real_gap() {
    let history = MemberHistory {
        id: 7,
        memberships: vec![
            HouseMembership {
                house: 1,
                start: date("1987-06-11"),
                end: Some(date("2025-01-01")),
            },
            HouseMembership {
                house: 1,
                start: date("2025-06-01"),
                end: None,
            },
        ],
    };
    let periods = history
        .service(date("2024-07-04"), date("2026-09-15"), true)
        .unwrap();
    assert_eq!(periods.len(), 2);
    assert_eq!(periods[0].source_start, date("1987-06-11"));
    assert_eq!(periods[0].served_from, date("2024-07-04"));
    assert_eq!(periods[0].end, Some(date("2025-01-01")));
    assert_eq!(periods[1].served_from, date("2025-06-01"));
}

#[test]
fn declaration_projection_matches_python_characterizations() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("tests/declaration_cases.json")).unwrap();
    for (index, case) in cases.as_array().unwrap().iter().enumerate() {
        let result = super::adapters::declarations::interpret(&case["source"]);
        if case["rejected"] == true {
            assert!(result.is_err(), "case {index} should reject");
        } else {
            let actual =
                serde_json::to_value(result.unwrap_or_else(|e| panic!("case {index}: {e}")))
                    .unwrap();
            assert_eq!(actual, case["draft"], "case {index}");
        }
    }
}
mod admin;
mod refresh;
mod storage;

#[test]
fn invalid_and_conflicting_service_histories_are_rejected() {
    let term = date("2024-07-04");
    for memberships in [
        vec![HouseMembership {
            house: 1,
            start: date("2025-06-01"),
            end: Some(date("2025-05-01")),
        }],
        vec![
            HouseMembership {
                house: 1,
                start: term,
                end: None,
            },
            HouseMembership {
                house: 1,
                start: date("2025-05-01"),
                end: None,
            },
        ],
        vec![HouseMembership {
            house: 1,
            start: term,
            end: Some(date("2025-01-01")),
        }],
    ] {
        assert!(
            MemberHistory { id: 1, memberships }
                .service(term, date("2026-09-15"), true)
                .is_err()
        );
    }
    let history = MemberHistory {
        id: 1,
        memberships: vec![HouseMembership {
            house: 1,
            start: date("2019-12-12"),
            end: Some(term),
        }],
    };
    assert!(
        history
            .service(term, date("2026-09-15"), false)
            .unwrap()
            .is_empty()
    );
}
mod notifications;
#[cfg(unix)]
mod process;
