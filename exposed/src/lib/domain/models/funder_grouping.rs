//! Funder groups and their direct evidence, built from the surviving stored identities.

use std::collections::{BTreeMap, BTreeSet};

use strum::IntoEnumIterator;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, strum::EnumIter)]
enum ReviewedFamily {
    Bbc,
    GbNews,
    PremierLeague,
}

struct ReviewedFamilyDefinition {
    key: &'static str,
    preferred_name: PreferredFunderName,
    names: &'static [&'static str],
}

impl ReviewedFamily {
    const fn definition(self) -> ReviewedFamilyDefinition {
        match self {
            Self::Bbc => ReviewedFamilyDefinition {
                key: "bbc",
                preferred_name: PreferredFunderName("BBC"),
                names: &[
                    "BBC",
                    "BBC Contributor Payments",
                    "BBC Home Services",
                    "BBC Studios",
                    "BBC Studios Ltd",
                    "BBC Wales Cymru",
                ],
            },
            Self::GbNews => ReviewedFamilyDefinition {
                key: "gb_news",
                preferred_name: PreferredFunderName("GB News"),
                names: &["GB News", "GB News Limited", "GB News Ltd"],
            },
            Self::PremierLeague => ReviewedFamilyDefinition {
                key: "premier_league",
                preferred_name: PreferredFunderName("Premier League"),
                names: &[
                    "Premier League",
                    "FA Premier League Ltd",
                    "The FA Premier League Ltd",
                    "The Premier League",
                    "Premier League International Holdings Ltd",
                ],
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PreferredFunderName(&'static str);

impl PreferredFunderName {
    pub(crate) const fn as_str(&self) -> &str {
        self.0
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

/// Invalid grouping input or competing reviewed families.
#[derive(Debug, Error)]
pub enum FunderGroupingError {
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

fn preferred_names(
    reviewed_members: Vec<(Uuid, ReviewedFamily)>,
    components: &mut Components,
) -> Result<Vec<FunderGroupName>, FunderGroupingError> {
    let mut reviewed_groups = BTreeMap::<Uuid, BTreeSet<ReviewedFamily>>::new();
    for (id, family) in reviewed_members {
        reviewed_groups
            .entry(components.root(id))
            .or_default()
            .insert(family);
    }
    let mut preferred_names = Vec::new();
    for (group_id, families) in reviewed_groups {
        if families.len() > 1 {
            let mut family_keys: Vec<_> = families
                .iter()
                .map(|family| family.definition().key.to_owned())
                .collect();
            family_keys.sort();
            return Err(FunderGroupingError::ConflictingReviewedFamilies {
                group_id: FunderId::new(group_id),
                family_keys,
            });
        }
        for family in families {
            let definition = family.definition();
            preferred_names.push(FunderGroupName {
                group_id: FunderId::new(group_id),
                preferred_name: definition.preferred_name,
                family_key: definition.key.to_owned(),
            });
        }
    }
    Ok(preferred_names)
}

/// Builds connected components from exact normalized names and reviewed canonical names.
/// The smallest UUID represents a component, including singletons.
///
/// # Errors
/// Rejects duplicate identities and components containing competing reviewed families.
pub fn build_funder_groups(
    funders: &[GroupingFunder],
) -> Result<FunderGroups, FunderGroupingError> {
    let mut ids = BTreeSet::new();
    for funder in funders {
        if !ids.insert(funder.id.value()) {
            return Err(FunderGroupingError::DuplicateFunder(funder.id.value()));
        }
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
    for family in ReviewedFamily::iter() {
        let definition = family.definition();
        let reviewed: BTreeSet<_> = definition
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
        reviewed_members.extend(members.iter().map(|funder| (funder.id.value(), family)));
        for (position, left) in members.iter().enumerate() {
            for right in &members[position + 1..] {
                components.join(left.id.value(), right.id.value());
                family_links.push(FunderFamilyLink {
                    left_funder_id: left.id,
                    right_funder_id: right.id,
                    family_key: definition.key.to_owned(),
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
    let preferred_names = preferred_names(reviewed_members, &mut components)?;
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
    fn reviewed_family_constants_are_coherent() {
        let mut keys = BTreeSet::new();
        let mut names = BTreeSet::new();
        for family in ReviewedFamily::iter() {
            let definition = family.definition();
            assert!(!definition.key.trim().is_empty());
            assert!(keys.insert(definition.key));
            let preferred_name = definition.preferred_name.as_str();
            assert!(!preferred_name.is_empty());
            assert_eq!(preferred_name, preferred_name.trim());
            assert!(!definition.names.is_empty());
            for name in definition.names {
                assert!(names.insert(NormalizedName::parse(name).unwrap()));
            }
        }
    }

    #[test]
    fn reviewed_alias_alone_does_not_assign_a_family() {
        let groups = build_funder_groups(&[funder(1, "External", &["BBC Studios Ltd"])]).unwrap();
        assert!(groups.preferred_names.is_empty());
        assert!(groups.family_links.is_empty());
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
    fn duplicate_funder_identities_are_rejected() {
        assert!(matches!(
            build_funder_groups(&[funder(1, "Alpha", &[]), funder(1, "Other", &[])]),
            Err(FunderGroupingError::DuplicateFunder(_))
        ));
    }
}
