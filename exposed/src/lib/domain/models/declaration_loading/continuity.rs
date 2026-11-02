use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use super::{DeclarationLoad, LoadFunder};
use crate::domain::models::{
    declaration_cleaning::{FunderObservationId, FundingEntryId},
    declaration_ingestion::DeclarationId,
    declaration_resolution::{AttributionBasis, AttributionIssue},
    entity_ingestion::EntityIngestionError,
};

#[derive(PartialEq, Eq)]
pub struct SelectedOccurrence {
    pub(crate) declaration: DeclarationId,
    pub(crate) observation: FunderObservationId,
    pub(crate) basis: AttributionBasis,
    pub(crate) parent: Option<DeclarationId>,
    pub(crate) issues: Vec<AttributionIssue>,
}

#[derive(PartialEq, Eq)]
pub struct FunderPresentation {
    pub(crate) name: String,
    pub(crate) aliases: BTreeSet<String>,
    pub(crate) kind: Option<String>,
    pub(crate) company_number: Option<String>,
}

impl From<&LoadFunder> for FunderPresentation {
    fn from(funder: &LoadFunder) -> Self {
        Self {
            name: funder.name.clone(),
            aliases: funder
                .aliases
                .iter()
                .chain(std::iter::once(&funder.name))
                .cloned()
                .collect(),
            kind: funder.kind.clone(),
            company_number: funder.company_number.clone(),
        }
    }
}

pub struct StoredFunderContinuity {
    pub(crate) id: Uuid,
    pub(crate) identity: Option<String>,
    pub(crate) presentation: FunderPresentation,
    pub(crate) complete_occurrences: Option<BTreeMap<FundingEntryId, SelectedOccurrence>>,
}

pub struct FunderReusePlan(BTreeMap<String, Uuid>);

impl FunderReusePlan {
    pub(crate) fn reused_id(&self, identity: &str) -> Option<Uuid> {
        self.0.get(identity).copied()
    }

    pub(crate) fn for_load(
        stored: &[StoredFunderContinuity],
        load: &DeclarationLoad,
    ) -> Result<Self, EntityIngestionError> {
        let incoming_keys = load
            .funders
            .iter()
            .map(|funder| funder.identity_id.as_str())
            .collect::<BTreeSet<_>>();
        let reserved = stored
            .iter()
            .filter(|funder| {
                funder
                    .identity
                    .as_deref()
                    .is_some_and(|key| incoming_keys.contains(key))
            })
            .map(|funder| funder.id)
            .collect::<BTreeSet<_>>();
        let exact_keys = stored
            .iter()
            .filter(|funder| reserved.contains(&funder.id))
            .filter_map(|funder| funder.identity.as_deref())
            .collect::<BTreeSet<_>>();
        let declarations = load
            .declarations
            .iter()
            .map(|row| row.source_declaration_id)
            .collect::<BTreeSet<_>>();
        let mut incoming = BTreeMap::<&str, BTreeMap<FundingEntryId, SelectedOccurrence>>::new();
        for entry in &load.funding_entries {
            if let (Some(identity), Some(observation), Some(basis)) = (
                &entry.identity_id,
                &entry.selected_observation_id,
                entry.attribution_basis,
            ) {
                incoming.entry(identity).or_default().insert(
                    serde_json::from_value(serde_json::Value::String(entry.source_id.clone()))
                        .map_err(|error: serde_json::Error| {
                            EntityIngestionError::DataError(error.to_string())
                        })?,
                    SelectedOccurrence {
                        declaration: entry.source_declaration_id,
                        observation: serde_json::from_value(serde_json::Value::String(
                            observation.clone(),
                        ))
                        .map_err(|error: serde_json::Error| {
                            EntityIngestionError::DataError(error.to_string())
                        })?,
                        basis,
                        parent: entry.selected_parent_declaration_id,
                        issues: entry.issues.clone(),
                    },
                );
            }
        }
        let mut candidates = BTreeMap::<&FundingEntryId, Vec<&StoredFunderContinuity>>::new();
        for funder in stored {
            if reserved.contains(&funder.id) {
                continue;
            }
            if let Some(occurrences) = &funder.complete_occurrences
                && occurrences
                    .values()
                    .all(|row| declarations.contains(&row.declaration))
                && let Some(first) = occurrences.keys().next()
            {
                candidates.entry(first).or_default().push(funder);
            }
        }
        let mut assignments = BTreeMap::new();
        for funder in &load.funders {
            if exact_keys.contains(funder.identity_id.as_str()) {
                continue;
            }
            let Some(occurrences) = incoming
                .get(funder.identity_id.as_str())
                .filter(|rows| !rows.is_empty())
            else {
                continue;
            };
            let presentation = FunderPresentation::from(funder);
            let matches = candidates
                .get(occurrences.keys().next().expect("nonempty occurrences"))
                .into_iter()
                .flatten()
                .filter(|candidate| {
                    candidate.presentation == presentation
                        && candidate.complete_occurrences.as_ref() == Some(occurrences)
                })
                .collect::<Vec<_>>();
            if let [candidate] = matches.as_slice() {
                assignments.insert(funder.identity_id.clone(), candidate.id);
            }
        }
        let mut counts = BTreeMap::<Uuid, usize>::new();
        for id in assignments.values() {
            *counts.entry(*id).or_default() += 1;
        }
        assignments.retain(|_, id| counts[id] == 1);
        Ok(Self(assignments))
    }
}
