//! Bounded candidate expansion and whole-component identity constraints.
use super::{
    AUTOMATIC_THRESHOLD, AddressMatchQuality, BTreeMap, BTreeSet, EntityIngestionError,
    FunderObservationId, IdentityBasis, Observation, ObservationResolution, PairDecision,
    PairDisposition, PairReason, ResolutionInput, ScoredPair, ScoringInput, invalid,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntityKind {
    Person,
    Organisation,
    Unknown,
}
fn kind(observation: &Observation) -> EntityKind {
    match observation
        .donor_kind
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("individual" | "person") => EntityKind::Person,
        Some(
            "company"
            | "charity"
            | "trade union"
            | "union"
            | "other organisation"
            | "organisation"
            | "unincorporated association"
            | "registered party"
            | "trust"
            | "limited liability partnership"
            | "friendly society"
            | "building society",
        ) => EntityKind::Organisation,
        _ => EntityKind::Unknown,
    }
}
pub(super) fn company(observation: &Observation) -> Option<String> {
    if observation
        .donor_kind
        .as_deref()
        .is_some_and(|kind| !kind.trim().eq_ignore_ascii_case("company"))
    {
        return None;
    }
    let number = observation
        .donor_company_number
        .as_ref()?
        .trim_matches(|c: char| c.is_ascii_whitespace())
        .to_ascii_uppercase();
    (number.len() == 8
        && number.bytes().all(|b| b.is_ascii_alphanumeric())
        && number.bytes().any(|b| b.is_ascii_digit()))
    .then_some(number)
}

fn strong(input: &ResolutionInput, edge: &PairDecision) -> bool {
    let a = &input.observations[&edge.left_funder_id];
    let b = &input.observations[&edge.right_funder_id];
    edge.probability >= AUTOMATIC_THRESHOLD
        && a.name_normalized.is_some()
        && a.name_normalized == b.name_normalized
        && a.address_match_quality == AddressMatchQuality::NumberedStreet
        && b.address_match_quality == AddressMatchQuality::NumberedStreet
        && a.address_normalized.is_some()
        && a.address_normalized == b.address_normalized
}

type CandidateEdges = BTreeMap<(FunderObservationId, FunderObservationId), PairDecision>;
fn expanded_edges(
    input: &ResolutionInput,
    scoring: &ScoringInput,
    pairs: Vec<ScoredPair>,
) -> Result<CandidateEdges, EntityIngestionError> {
    let mut edges = BTreeMap::new();
    for pair in pairs {
        for left in &scoring.members[&pair.left] {
            for right in &scoring.members[&pair.right] {
                if left == right {
                    continue;
                }
                let (left, right) = if left < right {
                    (left, right)
                } else {
                    (right, left)
                };
                let a = &input.observations[left];
                let b = &input.observations[right];
                if company(a).is_some() && company(b).is_some() {
                    continue;
                }
                edges
                    .entry((left.clone(), right.clone()))
                    .or_insert_with(|| PairDecision {
                        left_funder_id: left.clone(),
                        right_funder_id: right.clone(),
                        probability: pair.probability,
                        name_level: pair.name_level,
                        address_level: pair.address_level,
                        disposition: PairDisposition::Review,
                        reason: PairReason::InsufficientExactEvidence,
                    });
                if edges.len() > scoring.candidate_budget {
                    return Err(invalid(
                        "expanded observation pairs exceed candidate budget; raise candidate_budget explicitly",
                    ));
                }
            }
        }
    }
    Ok(edges)
}
struct Components {
    members: Vec<BTreeSet<FunderObservationId>>,
    group: BTreeMap<FunderObservationId, usize>,
}
impl Components {
    fn new(input: &ResolutionInput) -> Self {
        let mut components: Vec<BTreeSet<FunderObservationId>> = Vec::new();
        let mut anchors = BTreeMap::<String, usize>::new();
        let mut group = BTreeMap::new();
        for observation in input.observations.values() {
            let index = if let Some(number) = company(observation) {
                *anchors.entry(number).or_insert_with(|| {
                    components.push(BTreeSet::new());
                    components.len() - 1
                })
            } else {
                components.push(BTreeSet::new());
                components.len() - 1
            };
            components[index].insert(observation.funder_id.clone());
            group.insert(observation.funder_id.clone(), index);
        }
        Self {
            members: components,
            group,
        }
    }
    fn apply(&mut self, input: &ResolutionInput, edges: &mut CandidateEdges) {
        let mut candidate_companies = BTreeMap::<FunderObservationId, BTreeSet<String>>::new();
        for edge in edges.values().filter(|edge| strong(input, edge)) {
            for (unknown, known) in [
                (&edge.left_funder_id, &edge.right_funder_id),
                (&edge.right_funder_id, &edge.left_funder_id),
            ] {
                if company(&input.observations[unknown]).is_none()
                    && let Some(number) = company(&input.observations[known])
                {
                    candidate_companies
                        .entry(unknown.clone())
                        .or_default()
                        .insert(number);
                }
            }
        }
        let ambiguous = candidate_companies
            .iter()
            .filter(|(_, numbers)| numbers.len() > 1)
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        for edge in edges.values_mut() {
            let a = &input.observations[&edge.left_funder_id];
            let b = &input.observations[&edge.right_funder_id];
            if kind(a) != EntityKind::Unknown
                && kind(b) != EntityKind::Unknown
                && kind(a) != kind(b)
            {
                edge.disposition = PairDisposition::Rejected;
                edge.reason = PairReason::PersonOrganisationConflict;
                continue;
            }
            if a.address_match_quality == AddressMatchQuality::NumberedStreet
                && b.address_match_quality == AddressMatchQuality::NumberedStreet
                && a.address_normalized.is_some()
                && b.address_normalized.is_some()
                && a.address_normalized != b.address_normalized
            {
                edge.disposition = PairDisposition::Rejected;
                edge.reason = PairReason::FullAddressDisagreement;
                continue;
            }
            if !strong(input, edge) {
                continue;
            }
            if ambiguous.contains(&edge.left_funder_id) || ambiguous.contains(&edge.right_funder_id)
            {
                edge.reason = PairReason::CompetingCompanyAnchors;
                continue;
            }
            let left = self.group[&edge.left_funder_id];
            let right = self.group[&edge.right_funder_id];
            let ids = self.members[left]
                .union(&self.members[right])
                .cloned()
                .collect::<BTreeSet<_>>();
            let companies = ids
                .iter()
                .filter_map(|id| company(&input.observations[id]))
                .collect::<BTreeSet<_>>();
            let kinds = ids
                .iter()
                .map(|id| kind(&input.observations[id]))
                .collect::<Vec<_>>();
            if companies.len() > 1
                || (kinds.contains(&EntityKind::Person)
                    && kinds.contains(&EntityKind::Organisation))
            {
                edge.disposition = PairDisposition::Rejected;
                edge.reason = PairReason::ComponentIdentityConflict;
                continue;
            }
            if left != right {
                self.members[right].clear();
                self.members[left] = ids;
                for id in &self.members[left] {
                    self.group.insert(id.clone(), left);
                }
            }
            edge.disposition = PairDisposition::Accepted;
            edge.reason = PairReason::ExactNameFullAddressThreshold;
        }
    }
    fn outcomes(&self, input: &ResolutionInput, run_key: &str) -> Vec<ObservationResolution> {
        let mut observations = Vec::new();
        for observation in input.observations.values() {
            let ids = &self.members[self.group[&observation.funder_id]];
            let number = ids.iter().find_map(|id| company(&input.observations[id]));
            let (identity_id, identity_basis) = number.map_or_else(
                || {
                    if observation.name_normalized.is_none() {
                        (None, IdentityBasis::Unresolved)
                    } else {
                        let leader = ids
                            .first()
                            .expect("each observation belongs to a nonempty component")
                            .as_str();
                        (
                            Some(format!("run-local:{run_key}:{leader}")),
                            if ids.len() > 1 {
                                IdentityBasis::StatisticalLink
                            } else {
                                IdentityBasis::ProvisionalSingleton
                            },
                        )
                    }
                },
                |number| {
                    (
                        Some(format!("source-reported:companies-house:{number}")),
                        if company(observation).is_some() {
                            IdentityBasis::SourceReportedCompany
                        } else {
                            IdentityBasis::StatisticalLink
                        },
                    )
                },
            );
            observations.push(ObservationResolution {
                funder_id: observation.funder_id.clone(),
                identity_id,
                identity_basis,
            });
        }
        observations
    }
}
pub(super) fn resolve(
    input: &ResolutionInput,
    scoring: &ScoringInput,
    pairs: Vec<ScoredPair>,
    run_key: &str,
) -> Result<(Vec<ObservationResolution>, Vec<PairDecision>), EntityIngestionError> {
    let mut edges = expanded_edges(input, scoring, pairs)?;
    let mut components = Components::new(input);
    components.apply(input, &mut edges);
    Ok((
        components.outcomes(input, run_key),
        edges.into_values().collect(),
    ))
}
