use super::{ComparisonRow, EntityIngestionError, error};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Serialize)]
pub(super) enum CandidateRule {
    ExactName,
    ExactAddress,
    NamePrefix {
        characters: usize,
    },
    InitialSurname {
        excluded_left_suffixes: &'static [&'static str],
        surname_prefix_characters: usize,
    },
    SharedBlockingKey,
}
impl CandidateRule {
    pub(super) const ALL: [Self; 5] = [
        Self::ExactName,
        Self::ExactAddress,
        Self::NamePrefix { characters: 3 },
        Self::InitialSurname {
            surname_prefix_characters: 2,
            excluded_left_suffixes: &["limited", "ltd", "plc", "llp", "company", "inc", "and"],
        },
        Self::SharedBlockingKey,
    ];
    fn keys(self, row: &ComparisonRow) -> BTreeSet<String> {
        match self {
            Self::ExactName => BTreeSet::from([row.name.clone()]),
            Self::ExactAddress => row.address.iter().cloned().collect(),
            Self::NamePrefix { characters } => {
                BTreeSet::from([row.name.chars().take(characters).collect()])
            }
            Self::InitialSurname {
                surname_prefix_characters,
                ..
            } => BTreeSet::from([format!(
                "{}\0{}",
                row.name.chars().take(1).collect::<String>(),
                suffix(&row.name)
                    .chars()
                    .take(surname_prefix_characters)
                    .collect::<String>()
            )]),
            Self::SharedBlockingKey => row.blocking_keys.iter().cloned().collect(),
        }
    }
    fn accepts(self, left: &ComparisonRow) -> bool {
        match self {
            Self::InitialSurname {
                excluded_left_suffixes,
                ..
            } => !excluded_left_suffixes.contains(&suffix(&left.name)),
            Self::ExactName
            | Self::ExactAddress
            | Self::NamePrefix { .. }
            | Self::SharedBlockingKey => true,
        }
    }
}
fn suffix(name: &str) -> &str {
    let start = name
        .bytes()
        .rposition(|b| !b.is_ascii_lowercase())
        .map_or(0, |i| i + 1);
    &name[start..]
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub(super) struct Candidate {
    pub(super) left: usize,
    pub(super) right: usize,
}
#[derive(Default)]
struct Posting {
    all: Vec<usize>,
    unresolved: Vec<usize>,
}
struct CandidateSet {
    pairs: BTreeSet<Candidate>,
    budget: usize,
}
impl CandidateSet {
    fn insert(&mut self, pair: Candidate) -> Result<(), EntityIngestionError> {
        if self.pairs.contains(&pair) {
            return Ok(());
        }
        if self.pairs.len() == self.budget {
            return Err(error(format!(
                "candidate count exceeds budget {}",
                self.budget
            )));
        }
        self.pairs.insert(pair);
        Ok(())
    }
}

pub(super) fn bounded_candidates(
    rows: &[ComparisonRow],
    rules: &[CandidateRule],
    budget: usize,
) -> Result<BTreeSet<Candidate>, EntityIngestionError> {
    let mut candidates = CandidateSet {
        pairs: BTreeSet::new(),
        budget,
    };
    for &rule in rules {
        let mut index = BTreeMap::<String, Posting>::new();
        for (i, row) in rows.iter().enumerate() {
            for key in rule.keys(row) {
                let posting = index.entry(key).or_default();
                posting.all.push(i);
                if row.needs_resolution {
                    posting.unresolved.push(i);
                }
            }
        }
        for posting in index.values() {
            if posting.unresolved.is_empty() {
                continue;
            }
            for &left in &posting.all {
                if !rule.accepts(&rows[left]) {
                    continue;
                }
                let rights = if rows[left].needs_resolution {
                    &posting.all
                } else {
                    &posting.unresolved
                };
                let start = rights.partition_point(|&right| right <= left);
                for &right in &rights[start..] {
                    candidates.insert(Candidate { left, right })?;
                }
            }
        }
    }
    Ok(candidates.pairs)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(i: usize, unresolved: bool) -> ComparisonRow {
        ComparisonRow {
            key: format!("{i:08}"),
            name: "shared name".into(),
            address: None,
            blocking_keys: vec!["same".into(), "same".into()],
            needs_resolution: unresolved,
        }
    }
    #[test]
    fn duplicate_rules_and_tokens_preserve_exact_budget() {
        let rows = [row(0, true), row(1, true), row(2, true)];
        assert_eq!(
            bounded_candidates(&rows, &CandidateRule::ALL, 3)
                .unwrap()
                .len(),
            3
        );
        assert!(bounded_candidates(&rows, &CandidateRule::ALL, 2).is_err());
        assert!(bounded_candidates(&rows, &CandidateRule::ALL, 0).is_err());
    }
    #[test]
    fn anchored_groups_have_no_candidates() {
        let rows = (0..10_000).map(|i| row(i, false)).collect::<Vec<_>>();
        assert!(
            bounded_candidates(&rows, &CandidateRule::ALL, 0)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn refusal_does_not_insert_an_excess_pair() {
        let mut candidates = CandidateSet {
            pairs: BTreeSet::new(),
            budget: 2,
        };
        candidates.insert(Candidate { left: 0, right: 1 }).unwrap();
        candidates.insert(Candidate { left: 0, right: 2 }).unwrap();
        assert!(candidates.insert(Candidate { left: 0, right: 3 }).is_err());
        assert_eq!(candidates.pairs.len(), 2);
        let mut rows = (0..10_000).map(|i| row(i, false)).collect::<Vec<_>>();
        rows[0].needs_resolution = true;
        assert!(bounded_candidates(&rows, &CandidateRule::ALL, 2).is_err());
    }
    #[test]
    fn excluded_left_suffix_groups_have_no_candidates() {
        let rows = (0..10_000)
            .map(|i| {
                let mut row = row(i, true);
                row.name = format!(
                    "a{}x limited",
                    char::from_u32(0x1000 + u32::try_from(i).unwrap()).unwrap()
                );
                row.blocking_keys = vec![row.key.clone()];
                row
            })
            .collect::<Vec<_>>();
        assert!(
            bounded_candidates(&rows, &CandidateRule::ALL, 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn initial_surname_exclusion_uses_only_the_canonical_left() {
        let rule = [CandidateRule::InitialSurname {
            excluded_left_suffixes: &["limited"],
            surname_prefix_characters: 2,
        }];
        let mut left = row(0, true);
        let mut right = row(1, true);
        left.name = "alpha limited".into();
        right.name = "another lime".into();
        assert!(
            bounded_candidates(&[left.clone(), right.clone()], &rule, 1)
                .unwrap()
                .is_empty()
        );
        left.name = "another lime".into();
        right.name = "alpha limited".into();
        assert_eq!(
            bounded_candidates(&[left, right], &rule, 1).unwrap().len(),
            1
        );
    }

    #[test]
    fn unicode_prefix_empty_suffix_and_present_empty_address_are_keys() {
        let mut left = row(0, true);
        let mut right = row(1, false);
        left.name = "é🙂a left".into();
        right.name = "é🙂a right".into();
        assert_eq!(
            bounded_candidates(
                &[left.clone(), right.clone()],
                &[CandidateRule::NamePrefix { characters: 3 }],
                1
            )
            .unwrap()
            .len(),
            1
        );
        left.name = "a!".into();
        right.name = "a?".into();
        assert_eq!(
            bounded_candidates(&[left.clone(), right.clone()], &[CandidateRule::ALL[3]], 1)
                .unwrap()
                .len(),
            1
        );
        left.address = Some(String::new());
        right.address = Some(String::new());
        assert_eq!(
            bounded_candidates(
                &[left.clone(), right.clone()],
                &[CandidateRule::ExactAddress],
                1
            )
            .unwrap()
            .len(),
            1
        );
        right.address = None;
        assert!(
            bounded_candidates(&[left, right], &[CandidateRule::ExactAddress], 0)
                .unwrap()
                .is_empty()
        );
    }
}
