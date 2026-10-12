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
    pub(crate) preferred_names: Vec<FunderGroupName>,
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
    preferred_name: PreferredFunderName,
    names: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct PreferredFunderName(String);

impl TryFrom<String> for PreferredFunderName {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.trim();
        if value.is_empty() {
            return Err("preferred funder names must be nonempty");
        }
        Ok(Self(value.into()))
    }
}

impl PreferredFunderName {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) struct FunderGroupName {
    pub(crate) group_id: FunderId,
    pub(crate) preferred_name: PreferredFunderName,
    pub(crate) family_key: String,
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
    /// A component cannot select a name from competing reviewed families.
    #[error("funder group {group_id:?} contains conflicting reviewed families {family_keys:?}")]
    ConflictingReviewedFamilies {
        /// Stable representative of the ambiguous component.
        group_id: FunderId,
        /// Competing policy keys in lexical order.
        family_keys: Vec<String>,
    },
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
    fn preferred_names(
        &self,
        reviewed_members: Vec<(Uuid, usize)>,
        components: &mut Components,
    ) -> Result<Vec<FunderGroupName>, FunderGroupingError> {
        let mut reviewed_groups = BTreeMap::<Uuid, BTreeSet<usize>>::new();
        for (id, family_index) in reviewed_members {
            reviewed_groups
                .entry(components.root(id))
                .or_default()
                .insert(family_index);
        }
        let mut preferred_names = Vec::new();
        for (group_id, families) in reviewed_groups {
            if families.len() > 1 {
                let mut family_keys: Vec<_> = families
                    .iter()
                    .map(|index| self.families[*index].key.clone())
                    .collect();
                family_keys.sort();
                return Err(FunderGroupingError::ConflictingReviewedFamilies {
                    group_id: FunderId::new(group_id),
                    family_keys,
                });
            }
            for index in families {
                let family = &self.families[index];
                preferred_names.push(FunderGroupName {
                    group_id: FunderId::new(group_id),
                    preferred_name: family.preferred_name.clone(),
                    family_key: family.key.clone(),
                });
            }
        }
        Ok(preferred_names)
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
    let mut reviewed_members = Vec::new();
    for (family_index, family) in policy.families.iter().enumerate() {
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
        reviewed_members.extend(
            members
                .iter()
                .map(|funder| (funder.id.value(), family_index)),
        );
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
    let preferred_names = policy.preferred_names(reviewed_members, &mut components)?;
    let memberships = ids
        .iter()
        .map(|id| (FunderId::new(*id), FunderId::new(components.root(*id))))
        .collect();
    Ok(FunderGroups {
        memberships,
        alias_links,
        family_links,
        preferred_names,
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
    fn reviewed_names_follow_final_components_including_singletons() {
        let singleton = build_funder_groups(&[funder(3, "BBC Studios Ltd", &[])]).unwrap();
        assert_eq!(singleton.preferred_names[0].preferred_name.as_str(), "BBC");
        assert!(singleton.family_links.is_empty());
        let mut input = vec![
            funder(3, "BBC Studios Ltd", &["Bridge"]),
            funder(1, "External", &["Bridge"]),
        ];
        for _ in 0..2 {
            let groups = build_funder_groups(&input).unwrap();
            assert_eq!(
                groups.preferred_names[0].group_id.value(),
                Uuid::from_u128(1)
            );
            assert_eq!(groups.preferred_names[0].preferred_name.as_str(), "BBC");
            assert_eq!(groups.alias_links[0].left_alias, "Bridge");
            input.reverse();
        }
        assert_eq!(input[0].name, "BBC Studios Ltd");
    }

    #[test]
    fn competing_reviewed_families_reject_final_component_deterministically() {
        let mut input = vec![
            funder(3, "GB News Ltd", &["Bridge"]),
            funder(1, "BBC Studios Ltd", &["Bridge"]),
        ];
        for _ in 0..2 {
            match build_funder_groups(&input) {
                Err(FunderGroupingError::ConflictingReviewedFamilies {
                    group_id,
                    family_keys,
                }) => {
                    assert_eq!(group_id.value(), Uuid::from_u128(1));
                    assert_eq!(family_keys, ["bbc", "gb_news"]);
                }
                _ => panic!("expected reviewed family conflict"),
            }
            input.reverse();
        }
    }

    #[test]
    fn preferred_names_parse_nonempty_trimmed_values() {
        for value in ["", "  ", "\t\n"] {
            assert!(PreferredFunderName::try_from(value.to_owned()).is_err());
        }
        assert_eq!(
            PreferredFunderName::try_from(" BBC Studios ".to_owned())
                .unwrap()
                .as_str(),
            "BBC Studios"
        );
        assert!(
            FamilyPolicy::parse(r#"{"version":1,"families":[{"key":"bbc","names":["BBC"]}]}"#)
                .is_err()
        );
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
            r#"{"version":1,"families":[{"key":"","preferred_name":"BBC","names":["BBC"]}]}"#,
            r#"{"version":1,"families":[{"key":"a","preferred_name":"BBC","names":["BBC"]},{"key":"a","preferred_name":"BBC","names":["Other"]}]}"#,
            r#"{"version":1,"families":[{"key":"a","preferred_name":"BBC","names":["BBC"]},{"key":"b","preferred_name":"Other","names":[" bBc "]}]}"#,
            r#"{"version":1,"families":[{"key":"a","preferred_name":"BBC","names":[" "]}]}"#,
            r#"{"version":1,"families":[{"key":"a","preferred_name":"BBC","names":[]}]}"#,
        ] {
            assert!(FamilyPolicy::parse(input).is_err(), "{input}");
        }
        assert!(matches!(
            build_funder_groups(&[funder(1, "Alpha", &[]), funder(1, "Other", &[])]),
            Err(FunderGroupingError::DuplicateFunder(_))
        ));
    }
}
