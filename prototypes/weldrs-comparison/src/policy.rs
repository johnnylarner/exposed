include!(concat!(env!("OUT_DIR"), "/canonical.rs"));

mod entity_ingestion {
    #[derive(Debug, thiserror::Error)]
    pub enum EntityIngestionError {
        #[error("{0}")]
        DataError(String),
    }
}

use anyhow::Result;
use declaration_cleaning::{AddressMatchQuality, FunderRole};
use declaration_resolution::{
    Observation, Payment, ResolutionInput, ResolvedDeclarations, ScoredPair, ScoredPairs,
};
use funder_name::FunderNameFeatures;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const MEMBER: &str = "00000000-0000-0000-0000-000000000001";

pub(super) fn from_file(path: &str) -> Result<PolicyCase> {
    #[derive(serde::Deserialize)]
    struct Cleaned {
        observations: Vec<Observation>,
        payments: Vec<Payment>,
    }
    let cleaned: Cleaned = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    Ok(PolicyCase {
        name: "local_capture".into(),
        budget: 1_000_000,
        input: ResolutionInput::new(cleaned.observations, cleaned.payments, BTreeMap::new())?,
    })
}

pub(super) fn summary(result: &Value) -> Value {
    if result.get("error").is_some() {
        return result.clone();
    }
    let count = |array: &str, field: &str| {
        let mut counts = BTreeMap::<String, usize>::new();
        for row in result[array].as_array().unwrap() {
            *counts
                .entry(row[field].as_str().unwrap_or("none").to_owned())
                .or_default() += 1;
        }
        counts
    };
    json!({"observations":result["observations"].as_array().unwrap().len(),
        "payments":result["attributions"].as_array().unwrap().len(),
        "pair_decisions":result["pair_decisions"].as_array().unwrap().len(),
        "identity_bases":count("observations","identity_basis"),
        "dispositions":count("pair_decisions","disposition"),
        "attribution_statuses":count("attributions","attribution_status")})
}

pub(super) struct PolicyCase {
    pub name: String,
    pub budget: usize,
    input: ResolutionInput,
}

impl PolicyCase {
    pub fn rows(&self) -> Vec<super::Row> {
        self.input
            .scoring_input(self.budget)
            .rows
            .into_iter()
            .map(|r: declaration_resolution::ComparisonRow| super::Row {
                key: r.key,
                name: r.name,
                address: r.address,
                blocking_keys: r.blocking_keys,
                needs_resolution: r.needs_resolution,
            })
            .collect()
    }

    pub fn resolve(&self, pairs: &[super::Pair]) -> Value {
        let scoring = self.input.scoring_input(self.budget);
        let pairs = pairs
            .iter()
            .map(|p| ScoredPair {
                left: p.left.clone(),
                right: p.right.clone(),
                probability: p.probability,
                name_level: p.name_level,
                address_level: p.address_level,
            })
            .collect();
        let result =
            ScoredPairs::checked(pairs, json!({"prototype":true}), &scoring).and_then(|scores| {
                ResolvedDeclarations::from_scored(&self.input, &scoring, scores, "comparison-run")
            });
        match result {
            Ok(result) => {
                let decisions = result.pairs.iter().map(|p| json!({"left":p.left_funder_id,"right":p.right_funder_id,"disposition":p.disposition,"reason":p.reason})).collect::<Vec<_>>();
                json!({"observations":result.observations,"attributions":result.attributions,"pair_decisions":decisions})
            }
            Err(e) => json!({"error":e.to_string()}),
        }
    }
}

fn observation(
    id: u32,
    name: Option<&str>,
    address: Option<&str>,
    kind: Option<&str>,
    company: Option<&str>,
    role: FunderRole,
) -> Observation {
    let features = FunderNameFeatures::from_name(name);
    let role_name = match role {
        FunderRole::Donor => "donor",
        FunderRole::Payer => "payer",
        FunderRole::UltimatePayer => "ultimate_payer",
    };
    Observation {
        funder_id: serde_json::from_value(json!(format!(
            "{MEMBER}/{id}/1/declaration/{role_name}"
        )))
        .unwrap(),
        member_id: MEMBER.into(),
        declaration_id: id,
        register_id: 1,
        role,
        source_scope: "declaration".into(),
        source_pointer: "/versions/0/fields".into(),
        funding_entry_id: None,
        funding_ordinal: None,
        donor_kind: kind.map(str::to_owned),
        donor_company_number: company.map(str::to_owned),
        name_raw: features.name_raw,
        name_normalized: features.name_normalized,
        organisation_core: features.organisation_core,
        alias_normalized: features.alias_normalized,
        address_normalized: address.map(str::to_owned),
        address_match_quality: AddressMatchQuality::from_normalized(address),
    }
}

fn funding(mut observation: Observation, ordinal: u32) -> Observation {
    let role = match observation.role {
        FunderRole::Donor => "donor",
        FunderRole::Payer => "payer",
        FunderRole::UltimatePayer => "ultimate_payer",
    };
    let id = format!(
        "{MEMBER}/{}/1/funding/{ordinal}",
        observation.declaration_id
    );
    observation.funder_id = serde_json::from_value(json!(format!("{id}/{role}"))).unwrap();
    observation.funding_entry_id = Some(serde_json::from_value(json!(id)).unwrap());
    observation.funding_ordinal = Some(ordinal);
    observation.source_scope = "funding_entry".into();
    observation
}

fn payment(
    id: u32,
    ordinal: u32,
    parent: Option<u32>,
    flag: Option<bool>,
    observations: &[Observation],
) -> Payment {
    let mut payment = Payment {
        funding_entry_id: serde_json::from_value(json!(format!(
            "{MEMBER}/{id}/1/funding/{ordinal}"
        )))
        .unwrap(),
        member_id: MEMBER.into(),
        parliament_member_id: 1,
        declaration_id: id,
        register_id: 1,
        funding_ordinal: ordinal,
        parent_declaration_id: parent,
        is_ultimate_payer_different: flag,
        donor_funder_id: None,
        payer_funder_id: None,
        ultimate_payer_funder_id: None,
    };
    for observation in observations
        .iter()
        .filter(|o| o.funding_entry_id.as_ref() == Some(&payment.funding_entry_id))
    {
        match observation.role {
            FunderRole::Donor => payment.donor_funder_id = Some(observation.funder_id.clone()),
            FunderRole::Payer => payment.payer_funder_id = Some(observation.funder_id.clone()),
            FunderRole::UltimatePayer => {
                payment.ultimate_payer_funder_id = Some(observation.funder_id.clone())
            }
        }
    }
    payment
}

pub(super) fn cases() -> Result<Vec<PolicyCase>> {
    use FunderRole::{Donor, Payer, UltimatePayer};
    let mut cases = Vec::new();
    let mut add = |name: &str, observations, payments, budget| -> Result<()> {
        cases.push(PolicyCase {
            name: format!("policy_{name}"),
            budget,
            input: ResolutionInput::new(observations, payments, BTreeMap::new())?,
        });
        Ok(())
    };
    for (name, observations) in [
        (
            "statistical",
            vec![
                observation(
                    1,
                    Some("Example Charity"),
                    Some("1 road"),
                    None,
                    None,
                    Payer,
                ),
                observation(
                    2,
                    Some("Example Charity"),
                    Some("1 road"),
                    None,
                    None,
                    Payer,
                ),
            ],
        ),
        (
            "donor_missing_address",
            vec![
                observation(
                    1,
                    Some("Gary Lubner"),
                    Some("1 road"),
                    Some("Individual"),
                    None,
                    Donor,
                ),
                observation(
                    2,
                    Some("Gary Lubner"),
                    None,
                    Some("Individual"),
                    None,
                    Donor,
                ),
            ],
        ),
        (
            "donor_address_disagreement",
            vec![
                observation(
                    1,
                    Some("Gary Lubner"),
                    Some("1 road"),
                    Some("Individual"),
                    None,
                    Donor,
                ),
                observation(
                    2,
                    Some("Gary Lubner"),
                    Some("2 road"),
                    Some("Individual"),
                    None,
                    Donor,
                ),
            ],
        ),
        (
            "fuzzy_payer",
            vec![
                observation(1, Some("John Smith"), Some("1 road"), None, None, Payer),
                observation(2, Some("Jon Smith"), None, None, None, Payer),
            ],
        ),
        (
            "executive_annotation",
            vec![
                observation(
                    1,
                    Some("HSBC UK (Ian Stuart, CEO)"),
                    Some("1 centenary square"),
                    Some("Company"),
                    None,
                    Donor,
                ),
                observation(
                    2,
                    Some("HSBC UK Bank plc"),
                    Some("1 centenary square"),
                    None,
                    None,
                    Payer,
                ),
            ],
        ),
        (
            "alias",
            vec![
                observation(
                    1,
                    Some("Northstar Consulting trading as Fletchers"),
                    None,
                    None,
                    None,
                    Donor,
                ),
                observation(2, Some("Fletchers"), None, None, None, Payer),
            ],
        ),
        (
            "trade_union",
            vec![
                observation(
                    1,
                    Some("Unite the Union"),
                    None,
                    Some("Trade Union"),
                    None,
                    Donor,
                ),
                observation(
                    2,
                    Some("East Midlands Unite"),
                    None,
                    Some("Trade Union"),
                    None,
                    Donor,
                ),
                observation(
                    3,
                    Some("United Against Hunger"),
                    None,
                    Some("Company"),
                    None,
                    Donor,
                ),
            ],
        ),
        (
            "shared_address",
            vec![
                observation(
                    1,
                    Some("Carlton Club"),
                    Some("1 road"),
                    Some("Company"),
                    None,
                    Donor,
                ),
                observation(
                    2,
                    Some("Carlton Club Political Committee"),
                    Some("1 road"),
                    Some("Company"),
                    None,
                    Donor,
                ),
            ],
        ),
        (
            "competing_anchors",
            vec![
                observation(
                    1,
                    Some("Example Ltd"),
                    Some("1 road"),
                    Some("Company"),
                    Some("12345678"),
                    Donor,
                ),
                observation(
                    2,
                    Some("Example Ltd"),
                    Some("1 road"),
                    Some("Company"),
                    Some("87654321"),
                    Donor,
                ),
                observation(
                    3,
                    Some("Example Ltd"),
                    Some("1 road"),
                    Some("Company"),
                    None,
                    Donor,
                ),
            ],
        ),
        (
            "person_organisation",
            vec![
                observation(
                    1,
                    Some("John Smith"),
                    Some("1 road"),
                    Some("Individual"),
                    None,
                    Donor,
                ),
                observation(
                    2,
                    Some("John Smith"),
                    Some("1 road"),
                    Some("Company"),
                    None,
                    Donor,
                ),
            ],
        ),
        (
            "withheld",
            vec![
                observation(1, Some("Withheld"), None, None, None, Donor),
                observation(2, None, None, None, None, Payer),
            ],
        ),
    ] {
        add(name, observations, Vec::new(), 100)?;
    }
    let repeated = (1..=3)
        .map(|id| {
            observation(
                id,
                Some("Example Charity"),
                Some("1 road"),
                None,
                None,
                Payer,
            )
        })
        .collect::<Vec<_>>();
    add("repeated_budget_2", repeated.clone(), Vec::new(), 2)?;
    add("repeated_budget_3", repeated, Vec::new(), 3)?;
    for (name, parent, flag, ultimate) in [
        ("parent_payer", Some(1), Some(false), None),
        ("different_unnamed", Some(1), Some(true), None),
        ("donor_fallback", None, None, None),
        ("explicit_ultimate", Some(1), Some(false), Some("Acme Ltd")),
        ("withheld_ultimate", Some(1), Some(true), Some("Withheld")),
        ("missing_parent", Some(99), Some(false), None),
    ] {
        let mut observations = vec![observation(
            1,
            Some("Acme Ltd"),
            Some("1 road"),
            None,
            None,
            Payer,
        )];
        for ordinal in 0..2 {
            observations.push(funding(
                observation(2, Some("Local Donor"), None, None, None, Donor),
                ordinal,
            ));
            if let Some(ultimate) = ultimate {
                observations.push(funding(
                    observation(2, Some(ultimate), None, None, None, UltimatePayer),
                    ordinal,
                ));
            }
        }
        let payments = (0..2)
            .map(|ordinal| payment(2, ordinal, parent, flag, &observations))
            .collect();
        add(name, observations, payments, 100)?;
    }
    let observations = vec![
        funding(
            observation(1, Some("First Payer"), None, None, None, Payer),
            0,
        ),
        funding(
            observation(2, Some("Second Payer"), None, None, None, Payer),
            0,
        ),
    ];
    let payments = vec![
        payment(1, 0, Some(2), Some(false), &observations),
        payment(2, 0, Some(1), Some(false), &observations),
    ];
    add("parent_cycle", observations, payments, 100)?;
    Ok(cases)
}
