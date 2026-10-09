use super::identity::company;
use super::{
    AddressMatchQuality, BTreeMap, BTreeSet, EntityIngestionError, FunderObservationId, FunderRole,
    FundingEntryId, invalid,
};
use crate::domain::models::{
    declaration_cleaning::SourceScope, declaration_ingestion::DeclarationId,
    parliament_member::MemberId,
};
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
pub struct Observation {
    pub funder_id: FunderObservationId,
    #[serde(rename = "parliament_member_id")]
    pub member_id: MemberId,
    pub declaration_id: DeclarationId,
    pub register_id: u32,
    pub role: FunderRole,
    pub source_scope: String,
    pub source_pointer: String,
    pub funding_entry_id: Option<FundingEntryId>,
    pub funding_ordinal: Option<u32>,
    pub donor_kind: Option<String>,
    pub donor_company_number: Option<String>,
    pub name_raw: Option<String>,
    pub name_normalized: Option<String>,
    pub organisation_core: Option<String>,
    pub alias_normalized: Vec<String>,
    pub address_normalized: Option<String>,
    pub address_match_quality: AddressMatchQuality,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Payment {
    pub funding_entry_id: FundingEntryId,
    #[serde(rename = "parliament_member_id")]
    pub member_id: MemberId,
    pub declaration_id: DeclarationId,
    pub register_id: u32,
    pub funding_ordinal: u32,
    pub parent_declaration_id: Option<DeclarationId>,
    pub is_ultimate_payer_different: Option<bool>,
    pub donor_funder_id: Option<FunderObservationId>,
    pub payer_funder_id: Option<FunderObservationId>,
    pub ultimate_payer_funder_id: Option<FunderObservationId>,
}

/// Cleaned evidence whose identity and role references have been checked together.
pub struct ResolutionInput {
    pub(crate) observations: BTreeMap<FunderObservationId, Observation>,
    pub(crate) payments: BTreeMap<FundingEntryId, Payment>,
    pub(crate) digests: BTreeMap<String, String>,
}

impl ResolutionInput {
    pub(crate) fn new(
        observations: Vec<Observation>,
        payments: Vec<Payment>,
        digests: BTreeMap<String, String>,
    ) -> Result<Self, EntityIngestionError> {
        let mut input = Self {
            observations: BTreeMap::new(),
            payments: BTreeMap::new(),
            digests,
        };
        let mut declarations = BTreeMap::new();
        for payment in payments {
            check_payment(&payment, &mut declarations)?;
            if input
                .payments
                .insert(payment.funding_entry_id.clone(), payment)
                .is_some()
            {
                return Err(invalid("duplicate funding occurrence ID"));
            }
        }
        for observation in observations {
            check_observation(&observation, &input, &mut declarations)?;
            if input
                .observations
                .insert(observation.funder_id.clone(), observation)
                .is_some()
            {
                return Err(invalid("duplicate funder observation ID"));
            }
        }
        for payment in input.payments.values() {
            for (id, role) in [
                (&payment.donor_funder_id, FunderRole::Donor),
                (&payment.payer_funder_id, FunderRole::Payer),
                (&payment.ultimate_payer_funder_id, FunderRole::UltimatePayer),
            ] {
                if let Some(id) = id {
                    let observation = input
                        .observations
                        .get(id)
                        .ok_or_else(|| invalid("dangling payment role reference"))?;
                    if observation.role != role
                        || observation.funding_entry_id.as_ref() != Some(&payment.funding_entry_id)
                    {
                        return Err(invalid("payment role points to unrelated observation"));
                    }
                }
            }
        }
        for observation in input.observations.values() {
            if let Some(id) = &observation.funding_entry_id {
                let payment = &input.payments[id];
                let role_id = match observation.role {
                    FunderRole::Donor => &payment.donor_funder_id,
                    FunderRole::Payer => &payment.payer_funder_id,
                    FunderRole::UltimatePayer => &payment.ultimate_payer_funder_id,
                };
                if role_id.as_ref() != Some(&observation.funder_id) {
                    return Err(invalid("unreferenced funding role observation"));
                }
            }
        }
        Ok(input)
    }

    /// Groups repeated feature profiles without asserting that they identify the same funder.
    #[must_use]
    pub fn scoring_input(&self, candidate_budget: usize) -> ScoringInput {
        let mut groups = BTreeMap::<(String, Option<String>), Vec<FunderObservationId>>::new();
        for observation in self.observations.values() {
            if let Some(name) = &observation.name_normalized {
                groups
                    .entry((name.clone(), observation.address_normalized.clone()))
                    .or_default()
                    .push(observation.funder_id.clone());
            }
        }
        let mut rows = Vec::new();
        let mut members = BTreeMap::new();
        for (index, ((name, address), ids)) in groups.into_iter().enumerate() {
            let key = format!("profile-{index:08}");
            rows.push(ComparisonRow {
                key: key.clone(),
                name: name.clone(),
                address: address.clone(),
                blocking_keys: ids
                    .iter()
                    .flat_map(|id| blocking_keys(&self.observations[id]))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                needs_resolution: ids
                    .iter()
                    .any(|id| company(&self.observations[id]).is_none()),
            });
            members.insert(key.clone(), ids.clone());
            if ids.len() > 1
                && ids
                    .iter()
                    .any(|id| company(&self.observations[id]).is_none())
            {
                let copy = format!("{key}-repeat");
                rows.push(ComparisonRow {
                    key: copy.clone(),
                    name,
                    address,
                    blocking_keys: ids
                        .iter()
                        .flat_map(|id| blocking_keys(&self.observations[id]))
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                    needs_resolution: true,
                });
                members.insert(copy, ids);
            }
        }
        ScoringInput {
            candidate_budget,
            rows,
            members,
        }
    }
}

fn blocking_keys(observation: &Observation) -> BTreeSet<String> {
    let mut keys = observation
        .name_normalized
        .iter()
        .chain(observation.alias_normalized.iter())
        .map(|name| format!("funder-name:{name}"))
        .collect::<BTreeSet<_>>();
    let is_trade_union = observation
        .donor_kind
        .as_deref()
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("trade union"));
    let Some(raw) = observation.name_raw.as_deref() else {
        return keys;
    };
    let raw = raw.to_lowercase();
    if is_trade_union
        && raw
            .split(|c: char| !c.is_alphanumeric())
            .any(|token| token == "unite")
    {
        keys.insert("trade-union:unite".to_owned());
    }
    keys
}

fn root_scope(pointer: &str) -> bool {
    let parts = pointer.split('/').collect::<Vec<_>>();
    parts.len() >= 4
        && parts[0].is_empty()
        && parts[1] == "versions"
        && parts[2].parse::<u32>().is_ok()
        && parts[3] == "fields"
        && (parts.len() - 4).is_multiple_of(3)
        && parts[4..].chunks(3).all(|chunk| {
            chunk[0].parse::<u32>().is_ok()
                && chunk[1] == "values"
                && chunk[2].parse::<u32>().is_ok()
        })
}
pub(super) fn is_root(pointer: &str) -> bool {
    pointer.split('/').count() == 4
}

/// Feature profiles supplied to the statistical scorer.
pub struct ScoringInput {
    pub(crate) candidate_budget: usize,
    pub(crate) rows: Vec<ComparisonRow>,
    pub(super) members: BTreeMap<String, Vec<FunderObservationId>>,
}
#[derive(Clone)]
#[cfg_attr(test, derive(Deserialize))]
pub struct ComparisonRow {
    pub key: String,
    pub name: String,
    pub address: Option<String>,
    pub blocking_keys: Vec<String>,
    pub needs_resolution: bool,
}

/// Validated model scores, with provenance supplied by the scoring runtime.
pub struct ScoredPairs {
    pub(crate) pairs: Vec<ScoredPair>,
    pub(crate) model: serde_json::Value,
}
#[derive(Deserialize)]
pub struct ScoredPair {
    pub left: String,
    pub right: String,
    pub probability: f64,
    pub name_level: i32,
    pub address_level: i32,
}
impl ScoredPairs {
    pub(crate) fn checked(
        pairs: Vec<ScoredPair>,
        model: serde_json::Value,
        input: &ScoringInput,
    ) -> Result<Self, EntityIngestionError> {
        let mut seen = BTreeSet::new();
        if pairs.len() > input.candidate_budget {
            return Err(invalid("scorer exceeded candidate budget"));
        }
        for pair in &pairs {
            if pair.left >= pair.right
                || !input.members.contains_key(&pair.left)
                || !input.members.contains_key(&pair.right)
                || !seen.insert((&pair.left, &pair.right))
            {
                return Err(invalid(
                    "scorer returned duplicate, unordered, or unknown profile pair",
                ));
            }
            if !pair.probability.is_finite()
                || !(0.0..=1.0).contains(&pair.probability)
                || !(-1..=2).contains(&pair.name_level)
                || !(-1..=1).contains(&pair.address_level)
            {
                return Err(invalid(
                    "scorer returned invalid probability or comparison level",
                ));
            }
        }
        Ok(Self { pairs, model })
    }
}

type DeclarationMetadata = BTreeMap<(MemberId, DeclarationId), (u32, Option<DeclarationId>)>;
fn check_payment(
    payment: &Payment,
    declarations: &mut DeclarationMetadata,
) -> Result<(), EntityIngestionError> {
    if payment.register_id == 0 {
        return Err(invalid("zero register identity"));
    }
    let metadata = (payment.register_id, payment.parent_declaration_id);
    if declarations
        .insert((payment.member_id, payment.declaration_id), metadata)
        .is_some_and(|old| old != metadata)
    {
        return Err(invalid(
            "declaration metadata disagrees between cleaned occurrences",
        ));
    }
    let expected = SourceScope::new(
        payment.member_id,
        payment.declaration_id,
        payment.register_id,
    )
    .funding_entry(payment.funding_ordinal);
    if payment.funding_entry_id != expected {
        return Err(invalid(
            "funding occurrence ID disagrees with its source identity",
        ));
    }
    Ok(())
}
fn check_observation(
    observation: &Observation,
    input: &ResolutionInput,
    declarations: &mut DeclarationMetadata,
) -> Result<(), EntityIngestionError> {
    if observation.register_id == 0 {
        return Err(invalid("invalid cleaned observation identity"));
    }
    check_scope(observation, input)?;
    if observation.role != FunderRole::Donor
        && (observation.donor_kind.is_some() || observation.donor_company_number.is_some())
    {
        return Err(invalid("payer role carries donor metadata"));
    }
    if observation.address_match_quality
        != AddressMatchQuality::from_normalized(observation.address_normalized.as_deref())
    {
        return Err(invalid(
            "address matching quality disagrees with public evidence",
        ));
    }
    if observation
        .name_normalized
        .as_ref()
        .is_some_and(|name| name.trim().is_empty())
        || observation
            .address_normalized
            .as_ref()
            .is_some_and(|address| address.trim().is_empty())
    {
        return Err(invalid("empty normalized comparison evidence"));
    }
    let expected = observation.funding_entry_id.as_ref().map_or_else(
        || {
            SourceScope::new(
                observation.member_id,
                observation.declaration_id,
                observation.register_id,
            )
            .declaration_observation(observation.role)
        },
        |id| id.observation(observation.role),
    );
    if observation.funder_id != expected {
        return Err(invalid(
            "funder observation ID disagrees with source scope and role",
        ));
    }
    let metadata = declarations
        .entry((observation.member_id, observation.declaration_id))
        .or_insert((observation.register_id, None));
    if metadata.0 != observation.register_id {
        return Err(invalid("observation declaration register mismatch"));
    }
    Ok(())
}
fn check_scope(
    observation: &Observation,
    input: &ResolutionInput,
) -> Result<(), EntityIngestionError> {
    let root = root_scope(&observation.source_pointer);
    if !root {
        return Err(invalid("malformed cleaned source pointer"));
    }
    match (
        observation.source_scope.as_str(),
        &observation.funding_entry_id,
        observation.funding_ordinal,
    ) {
        ("declaration", None, None) if is_root(&observation.source_pointer) => {}
        ("funding_entry", Some(id), Some(ordinal)) => {
            let payment = input
                .payments
                .get(id)
                .ok_or_else(|| invalid("observation has a dangling funding occurrence"))?;
            if payment.member_id != observation.member_id
                || payment.declaration_id != observation.declaration_id
                || payment.register_id != observation.register_id
                || payment.funding_ordinal != ordinal
            {
                return Err(invalid(
                    "observation scope disagrees with funding occurrence",
                ));
            }
        }
        _ => return Err(invalid("invalid observation source scope")),
    }
    Ok(())
}
