//! Funder groups and their direct evidence, built from the surviving stored identities.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

use super::{
    entity_details::{FunderAliasLink, FunderFamilyLink},
    funder::FunderId,
};

/// Canonical name and recorded aliases for one persisted identity.
pub struct GroupingFunder {
    /// Persistent identity.
    pub id: FunderId,
    /// Canonical source name.
    pub name: String,
    /// Exact recorded spellings.
    pub aliases: Vec<String>,
}

/// Complete replacement for the stored group membership and evidence.
pub struct FunderGroups {
    /// Every input identity belongs to exactly one component.
    pub(crate) memberships: Vec<(FunderId, FunderId)>,
    /// Direct equal-name evidence, without inferred transitive links.
    pub(crate) alias_links: Vec<FunderAliasLink>,
    /// Direct reviewed-family evidence, kept separate from equal-name evidence.
    pub(crate) family_links: Vec<FunderFamilyLink>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FamilyPolicy {
    version: PolicyVersion,
    families: Vec<ReviewedFamily>,
}

#[derive(Deserialize)]
#[serde(try_from = "u32")]
enum PolicyVersion {
    V1,
}

impl TryFrom<u32> for PolicyVersion {
    type Error = &'static str;

    fn try_from(version: u32) -> Result<Self, Self::Error> {
        match version {
            1 => Ok(Self::V1),
            _ => Err("unsupported reviewed funder family policy version"),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedFamily {
    key: String,
    names: Vec<String>,
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
struct NormalizedName(String);

impl NormalizedName {
    fn parse(name: &str) -> Option<Self> {
        let name = name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        (!name.is_empty()).then_some(Self(name))
    }
}

/// Invalid grouping input or reviewed policy.
#[derive(Debug, Error)]
pub enum FunderGroupingError {
    /// The versioned resource does not match its schema.
    #[error("invalid reviewed funder policy: {0}")]
    PolicyDecode(#[from] serde_json::Error),
    /// Policy names and keys must identify one reviewed family.
    #[error("invalid reviewed funder policy: {0}")]
    Policy(String),
    /// A persisted identity must occur exactly once in a snapshot.
    #[error("duplicate funder identity {0}")]
    DuplicateFunder(Uuid),
}

impl FamilyPolicy {
    fn parse(input: &str) -> Result<Self, FunderGroupingError> {
        let policy: Self = serde_json::from_str(input)?;
        let mut keys = BTreeSet::new();
        let mut names = BTreeSet::new();
        for family in &policy.families {
            if family.key.trim().is_empty() || !keys.insert(&family.key) {
                return Err(FunderGroupingError::Policy(
                    "family keys must be nonempty and unique".into(),
                ));
            }
            if family.names.is_empty() {
                return Err(FunderGroupingError::Policy(
                    "a reviewed family must have names".into(),
                ));
            }
            for name in &family.names {
                let normalized = NormalizedName::parse(name).ok_or_else(|| {
                    FunderGroupingError::Policy("reviewed names must be nonempty".into())
                })?;
                if !names.insert(normalized) {
                    return Err(FunderGroupingError::Policy(
                        "each normalized reviewed name must occur once".into(),
                    ));
                }
            }
        }
        Ok(policy)
    }
}

/// Builds connected components from exact normalized names and reviewed canonical names.
/// The smallest UUID represents a component, including singletons.
///
/// # Errors
/// Rejects duplicate identities and malformed or ambiguous reviewed policy.
pub fn build_funder_groups(
    funders: &[GroupingFunder],
) -> Result<FunderGroups, FunderGroupingError> {
    let policy = FamilyPolicy::parse(include_str!("funder_families.v1.json"))?;
    let mut ids = BTreeSet::new();
    for funder in funders {
        if !ids.insert(funder.id.value()) {
            return Err(FunderGroupingError::DuplicateFunder(funder.id.value()));
        }
    }
    match policy.version {
        PolicyVersion::V1 => {}
    }
    let mut components = Components::new(funders);
    let mut names = BTreeMap::<NormalizedName, BTreeMap<Uuid, BTreeSet<&str>>>::new();
    for funder in funders {
        for name in std::iter::once(&funder.name).chain(&funder.aliases) {
            if let Some(normalized) = NormalizedName::parse(name) {
                names
                    .entry(normalized)
                    .or_default()
                    .entry(funder.id.value())
                    .or_default()
                    .insert(name);
            }
        }
    }
    let mut alias_links = Vec::new();
    for (normalized, identities) in names {
        let identities: Vec<_> = identities.into_iter().collect();
        for (position, (left_id, left_names)) in identities.iter().enumerate() {
            for (right_id, right_names) in &identities[position + 1..] {
                components.join(*left_id, *right_id);
                for left in left_names {
                    for right in right_names {
                        alias_links.push(FunderAliasLink {
                            left_funder_id: FunderId::new(*left_id),
                            right_funder_id: FunderId::new(*right_id),
                            normalized_alias: normalized.0.clone(),
                            left_alias: (*left).into(),
                            right_alias: (*right).into(),
                        });
                    }
                }
            }
        }
    }
    let mut family_links = Vec::new();
    for family in policy.families {
        let reviewed: BTreeSet<_> = family
            .names
            .iter()
            .filter_map(|name| NormalizedName::parse(name))
            .collect();
        let mut members: Vec<_> = funders
            .iter()
            .filter(|funder| {
                NormalizedName::parse(&funder.name).is_some_and(|name| reviewed.contains(&name))
            })
            .collect();
        members.sort_by_key(|funder| funder.id.value());
        for (position, left) in members.iter().enumerate() {
            for right in &members[position + 1..] {
                components.join(left.id.value(), right.id.value());
                family_links.push(FunderFamilyLink {
                    left_funder_id: left.id,
                    right_funder_id: right.id,
                    family_key: family.key.clone(),
                    left_name: left.name.clone(),
                    right_name: right.name.clone(),
                });
            }
        }
    }
    family_links.sort_by_key(|link| {
        (
            link.family_key.clone(),
            link.left_funder_id.value(),
            link.right_funder_id.value(),
        )
    });
    let memberships = ids
        .iter()
        .map(|id| (FunderId::new(*id), FunderId::new(components.root(*id))))
        .collect();
    Ok(FunderGroups {
        memberships,
        alias_links,
        family_links,
    })
}

struct Components {
    parents: BTreeMap<Uuid, Uuid>,
}

impl Components {
    fn new(funders: &[GroupingFunder]) -> Self {
        Self {
            parents: funders
                .iter()
                .map(|funder| (funder.id.value(), funder.id.value()))
                .collect(),
        }
    }

    fn root(&mut self, id: Uuid) -> Uuid {
        let parent = self.parents[&id];
        if parent == id {
            return id;
        }
        let root = self.root(parent);
        self.parents.insert(id, root);
        root
    }

    fn join(&mut self, left: Uuid, right: Uuid) {
        let left = self.root(left);
        let right = self.root(right);
        self.parents.insert(left.max(right), left.min(right));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn funder(id: u128, name: &str, aliases: &[&str]) -> GroupingFunder {
        GroupingFunder {
            id: FunderId::new(Uuid::from_u128(id)),
            name: name.into(),
            aliases: aliases.iter().map(|name| (*name).into()).collect(),
        }
    }

    #[test]
    fn connected_components_keep_direct_evidence_and_split_after_bridge_removal() {
        let mut funders = vec![
            funder(3, "Gamma", &["Bridge two", "ALPHA"]),
            funder(1, "Alpha", &["Bridge one"]),
            funder(2, "Beta", &["  BRIDGE\tONE  ", "Bridge two"]),
            funder(4, "Separate", &["  "]),
        ];
        let groups = build_funder_groups(&funders).unwrap();
        assert_eq!(
            groups
                .memberships
                .iter()
                .map(|(id, group)| (id.value().as_u128(), group.value().as_u128()))
                .collect::<Vec<_>>(),
            [(1, 1), (2, 1), (3, 1), (4, 4)]
        );
        assert_eq!(groups.alias_links.len(), 3);
        assert!(groups.family_links.is_empty());
        funders[0].aliases.clear();
        let groups = build_funder_groups(&funders).unwrap();
        assert_eq!(groups.memberships[2].1.value(), Uuid::from_u128(3));
        assert_eq!(groups.alias_links.len(), 1);
    }

    #[test]
    fn reviewed_policy_rejects_ambiguous_names_empty_keys_and_unknown_versions() {
        for input in [
            r#"{"version":2,"families":[]}"#,
            r#"{"version":1,"families":[{"key":"","names":["BBC"]}]}"#,
            r#"{"version":1,"families":[{"key":"a","names":["BBC"]},{"key":"a","names":["Other"]}]}"#,
            r#"{"version":1,"families":[{"key":"a","names":["BBC"]},{"key":"b","names":[" bBc "]}]}"#,
            r#"{"version":1,"families":[{"key":"a","names":[" "]}]}"#,
            r#"{"version":1,"families":[{"key":"a","names":[]}]}"#,
        ] {
            assert!(FamilyPolicy::parse(input).is_err(), "{input}");
        }
        assert!(matches!(
            build_funder_groups(&[funder(1, "Alpha", &[]), funder(1, "Other", &[])]),
            Err(FunderGroupingError::DuplicateFunder(_))
        ));
    }
}
