use super::{
    AUTOMATIC_THRESHOLD, AddressMatchQuality, BTreeMap, BTreeSet, EntityIngestionError,
    FunderObservationId, FunderRole, IdentityBasis, Observation, ObservationResolution,
    PairDecision, PairDisposition, PairReason, ResolutionInput, ScoredPair, ScoringInput, invalid,
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

#[derive(Clone, Copy)]
enum LinkEvidence {
    Statistical,
    DonorName,
}
#[derive(Clone, Copy)]
enum EdgeEligibility {
    Link(LinkEvidence),
    Review,
    Reject(PairReason),
}
struct CandidateEdge {
    decision: PairDecision,
    eligibility: EdgeEligibility,
}
fn classify(a: &Observation, b: &Observation, probability: f64) -> EdgeEligibility {
    if kind(a) != EntityKind::Unknown && kind(b) != EntityKind::Unknown && kind(a) != kind(b) {
        return EdgeEligibility::Reject(PairReason::PersonOrganisationConflict);
    }
    let exact_name = a.name_normalized.is_some() && a.name_normalized == b.name_normalized;
    let full_addresses = a.address_match_quality == AddressMatchQuality::NumberedStreet
        && b.address_match_quality == AddressMatchQuality::NumberedStreet
        && a.address_normalized.is_some()
        && b.address_normalized.is_some();
    if exact_name
        && full_addresses
        && a.address_normalized == b.address_normalized
        && probability >= AUTOMATIC_THRESHOLD
    {
        EdgeEligibility::Link(LinkEvidence::Statistical)
    } else if exact_name && a.role == FunderRole::Donor && b.role == FunderRole::Donor {
        EdgeEligibility::Link(LinkEvidence::DonorName)
    } else if full_addresses && a.address_normalized != b.address_normalized {
        EdgeEligibility::Reject(PairReason::FullAddressDisagreement)
    } else {
        EdgeEligibility::Review
    }
}
type CandidateEdges = BTreeMap<(FunderObservationId, FunderObservationId), CandidateEdge>;
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
                    .or_insert_with(|| CandidateEdge {
                        eligibility: classify(a, b, pair.probability),
                        decision: PairDecision {
                            left_funder_id: left.clone(),
                            right_funder_id: right.clone(),
                            probability: pair.probability,
                            name_level: pair.name_level,
                            address_level: pair.address_level,
                            disposition: PairDisposition::Review,
                            reason: PairReason::InsufficientExactEvidence,
                        },
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
        let mut unanchored = Self::new(input);
        for edge in edges
            .values()
            .filter(|edge| matches!(edge.eligibility, EdgeEligibility::Link(_)))
        {
            let edge = &edge.decision;
            if company(&input.observations[&edge.left_funder_id]).is_none()
                && company(&input.observations[&edge.right_funder_id]).is_none()
            {
                unanchored.join(&edge.left_funder_id, &edge.right_funder_id);
            }
        }
        let mut candidate_companies = BTreeMap::<usize, BTreeSet<String>>::new();
        for candidate in edges
            .values()
            .filter(|edge| matches!(edge.eligibility, EdgeEligibility::Link(_)))
        {
            let edge = &candidate.decision;
            for (unknown, known) in [
                (&edge.left_funder_id, &edge.right_funder_id),
                (&edge.right_funder_id, &edge.left_funder_id),
            ] {
                if company(&input.observations[unknown]).is_none()
                    && let Some(number) = company(&input.observations[known])
                {
                    candidate_companies
                        .entry(unanchored.group[unknown])
                        .or_default()
                        .insert(number);
                }
            }
        }
        for candidate in edges.values_mut() {
            let edge = &mut candidate.decision;
            let evidence = match candidate.eligibility {
                EdgeEligibility::Reject(reason) => {
                    edge.disposition = PairDisposition::Rejected;
                    edge.reason = reason;
                    continue;
                }
                EdgeEligibility::Review => continue,
                EdgeEligibility::Link(evidence) => evidence,
            };
            let ambiguous_attachment = [
                (&edge.left_funder_id, &edge.right_funder_id),
                (&edge.right_funder_id, &edge.left_funder_id),
            ]
            .iter()
            .any(|(unknown, known)| {
                company(&input.observations[*unknown]).is_none()
                    && company(&input.observations[*known]).is_some()
                    && candidate_companies
                        .get(&unanchored.group[*unknown])
                        .is_some_and(|numbers| numbers.len() > 1)
            });
            if ambiguous_attachment {
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
            self.join(&edge.left_funder_id, &edge.right_funder_id);
            edge.disposition = PairDisposition::Accepted;
            edge.reason = match evidence {
                LinkEvidence::Statistical => PairReason::ExactNameFullAddressThreshold,
                LinkEvidence::DonorName => PairReason::ExactDonorName,
            };
        }
    }
    fn join(&mut self, left_id: &FunderObservationId, right_id: &FunderObservationId) {
        let left = self.group[left_id];
        let right = self.group[right_id];
        if left != right {
            let members = std::mem::take(&mut self.members[right]);
            for id in &members {
                self.group.insert(id.clone(), left);
            }
            self.members[left].extend(members);
        }
    }
    fn outcomes(
        &self,
        input: &ResolutionInput,
        edges: &CandidateEdges,
        run_key: &str,
    ) -> Vec<ObservationResolution> {
        let mut statistical = Self::new(input);
        let mut statistical_groups = BTreeSet::new();
        for candidate in edges.values() {
            if candidate.decision.disposition == PairDisposition::Accepted
                && matches!(
                    candidate.eligibility,
                    EdgeEligibility::Link(LinkEvidence::Statistical)
                )
            {
                let edge = &candidate.decision;
                statistical.join(&edge.left_funder_id, &edge.right_funder_id);
                statistical_groups.insert(self.group[&edge.left_funder_id]);
            }
        }
        let mut observations = Vec::new();
        for observation in input.observations.values() {
            let ids = &self.members[self.group[&observation.funder_id]];
            let statistical_connected = ids
                .iter()
                .map(|id| statistical.group[id])
                .collect::<BTreeSet<_>>()
                .len()
                == 1;
            let component_basis = if statistical_connected {
                IdentityBasis::StatisticalLink
            } else if statistical_groups.contains(&self.group[&observation.funder_id]) {
                IdentityBasis::StatisticalAndDonorNameLink
            } else {
                IdentityBasis::DonorNameLink
            };
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
                                component_basis
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
                            component_basis
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
        components.outcomes(input, &edges, run_key),
        edges.into_values().map(|edge| edge.decision).collect(),
    ))
}
