use super::*;

fn observation(
    id: u32,
    name: Option<&str>,
    address: Option<&str>,
    kind: Option<&str>,
    company: Option<&str>,
) -> Observation {
    Observation {
        funder_id: serde_json::from_value(serde_json::json!(format!(
            "00000000-0000-0000-0000-000000000001/{id}/1/declaration/donor"
        )))
        .unwrap(),
        member_id: "00000000-0000-0000-0000-000000000001".into(),
        declaration_id: id,
        register_id: 1,
        role: FunderRole::Donor,
        source_scope: "declaration".into(),
        source_pointer: "/versions/0/fields".into(),
        funding_entry_id: None,
        funding_ordinal: None,
        donor_kind: kind.map(str::to_owned),
        donor_company_number: company.map(str::to_owned),
        name_raw: name.map(str::to_owned),
        name_normalized: name.map(str::to_owned),
        organisation_core: name.map(str::to_owned),
        alias_normalized: Vec::new(),
        address_normalized: address.map(str::to_owned),
        address_match_quality: AddressMatchQuality::from_normalized(address),
    }
}
fn input(observations: Vec<Observation>) -> ResolutionInput {
    ResolutionInput::new(observations, Vec::new(), BTreeMap::new()).unwrap()
}
fn resolve(input: &ResolutionInput, probability: f64) -> ResolvedDeclarations {
    let scoring = input.scoring_input(100);
    let pairs = scoring
        .rows
        .iter()
        .enumerate()
        .flat_map(|(index, left)| {
            scoring.rows[index + 1..]
                .iter()
                .map(move |right| ScoredPair {
                    left: left.key.clone(),
                    right: right.key.clone(),
                    probability,
                    name_level: 2,
                    address_level: 1,
                })
        })
        .collect();
    let scored =
        ScoredPairs::checked(pairs, serde_json::json!({"calibrated":false}), &scoring).unwrap();
    ResolvedDeclarations::from_scored(input, &scoring, scored, "test-run").unwrap()
}
fn payer(mut observation: Observation) -> Observation {
    observation.role = FunderRole::Payer;
    observation.donor_kind = None;
    observation.donor_company_number = None;
    observation.funder_id = serde_json::from_value(serde_json::json!(
        observation.funder_id.as_str().replace("/donor", "/payer")
    ))
    .unwrap();
    observation
}
#[test]
fn exact_donor_names_link_without_address_or_threshold() {
    for addresses in [
        (None, None),
        (Some("1 road"), Some("2 road")),
        (Some("london"), None),
    ] {
        let result = resolve(
            &input(vec![
                observation(
                    1,
                    Some("gary lubner"),
                    addresses.0,
                    Some("Individual"),
                    None,
                ),
                observation(
                    2,
                    Some("gary lubner"),
                    addresses.1,
                    Some("Individual"),
                    None,
                ),
            ]),
            0.01,
        );
        assert_eq!(
            result.observations[0].identity_id,
            result.observations[1].identity_id
        );
        assert!(
            result
                .observations
                .iter()
                .all(|row| matches!(row.identity_basis, IdentityBasis::DonorNameLink))
        );
        assert_eq!(result.pairs[0].probability.to_bits(), 0.01_f64.to_bits());
        assert!(matches!(result.pairs[0].reason, PairReason::ExactDonorName));
        assert_eq!(result.pairs[0].disposition, PairDisposition::Accepted);
    }
}
#[test]
fn fuzzy_missing_and_other_role_names_do_not_use_donor_rule() {
    for (left, right) in [
        (
            observation(1, Some("john smith"), None, None, None),
            observation(2, Some("jon smith"), None, None, None),
        ),
        (
            observation(1, None, None, None, None),
            observation(2, None, None, None, None),
        ),
        (
            observation(1, Some("john smith"), None, None, None),
            payer(observation(2, Some("john smith"), None, None, None)),
        ),
    ] {
        let result = resolve(&input(vec![left, right]), 0.9999);
        assert!(
            result
                .pairs
                .iter()
                .all(|pair| pair.disposition == PairDisposition::Review)
        );
        assert!(result.observations.iter().all(|row| matches!(
            row.identity_basis,
            IdentityBasis::ProvisionalSingleton | IdentityBasis::Unresolved
        )));
    }
}

#[test]
fn extracted_hsbc_name_and_punctuation_normalized_address_link_across_roles() {
    let mut ceo = observation(
        1,
        Some("HSBC UK (Ian Stuart, CEO)"),
        Some("1 Centenary Square, Birmingham, B1 1HQ"),
        Some("Company"),
        None,
    );
    ceo.name_raw = Some("HSBC UK (Ian Stuart, CEO)".into());
    let mut bank = observation(
        2,
        Some("HSBC UK Bank plc"),
        Some("1 Centenary Square Birmingham B1 1HQ"),
        Some("Company"),
        None,
    );
    bank.name_raw = Some("HSBC UK Bank plc".into());
    let payer_observation = payer(bank.clone());
    let result = resolve(&input(vec![ceo, payer_observation]), 0.08);
    assert_eq!(
        result.observations[0].identity_id,
        result.observations[1].identity_id
    );
    assert!(result.pairs.iter().any(|pair| {
        pair.disposition == PairDisposition::Accepted
            && matches!(pair.reason, PairReason::ExtractedNameEvidence)
    }));
}

#[test]
fn explicit_cleaner_aliases_are_positive_resolution_evidence() {
    let mut legal_name = observation(1, Some("Fletcher Legal Ltd"), None, None, None);
    legal_name.alias_normalized = vec!["fletchers".into()];
    let brand = observation(2, Some("fletchers"), None, None, None);
    let result = resolve(&input(vec![legal_name, brand]), 0.01);
    assert_eq!(
        result.observations[0].identity_id,
        result.observations[1].identity_id
    );
    assert!(result.pairs.iter().any(|pair| {
        pair.disposition == PairDisposition::Accepted
            && matches!(pair.reason, PairReason::ExtractedNameEvidence)
    }));
}

#[test]
fn unite_trade_union_variants_link_but_similarly_spelled_entities_do_not() {
    let names = [
        "Unite Union",
        "Unite the Union",
        "Unite West Midlands",
        "East Midlands Unite the Union",
        "UNITE The Union (West Midlands)",
        "Unite the Union Parliamentary Staff Branch",
    ];
    let unions = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            observation(
                index as u32 + 1,
                Some(name),
                None,
                Some("Trade Union"),
                None,
            )
        })
        .collect::<Vec<_>>();
    let result = resolve(&input(unions), 0.01);
    assert!(
        result
            .observations
            .iter()
            .all(|row| row.identity_id == result.observations[0].identity_id)
    );
    assert!(
        result
            .pairs
            .iter()
            .filter(|pair| pair.disposition == PairDisposition::Accepted)
            .all(|pair| matches!(pair.reason, PairReason::TradeUnionFamily))
    );
    assert!(
        result
            .observations
            .iter()
            .all(|row| matches!(row.identity_basis, IdentityBasis::TradeUnionFamilyLink))
    );

    let separate = resolve(
        &input(vec![
            observation(20, Some("Unite"), None, Some("Trade Union"), None),
            observation(
                21,
                Some("United Against Malnutrition and Hunger"),
                None,
                Some("Charity"),
                None,
            ),
        ]),
        0.9999,
    );
    assert_ne!(
        separate.observations[0].identity_id,
        separate.observations[1].identity_id
    );

    let committee = resolve(
        &input(vec![
            observation(
                30,
                Some("Carlton Club"),
                Some("69 St James's Street, London SW1A 1PJ"),
                Some("Unincorporated association"),
                None,
            ),
            observation(
                31,
                Some("Carlton Club Political Committee"),
                Some("69 St. James's Street London SW1A 1PJ"),
                Some("Unincorporated association"),
                None,
            ),
        ]),
        0.9999,
    );
    assert_ne!(
        committee.observations[0].identity_id,
        committee.observations[1].identity_id
    );
}
#[test]
fn statistical_support_takes_precedence_and_mixed_components_are_honest() {
    let result = resolve(
        &input(vec![
            observation(1, Some("john smith"), Some("1 road"), None, None),
            observation(2, Some("john smith"), Some("1 road"), None, None),
            observation(3, Some("john smith"), None, None, None),
        ]),
        0.9999,
    );
    assert!(result.observations.iter().all(|row| matches!(
        row.identity_basis,
        IdentityBasis::StatisticalAndDonorNameLink
    )));
    assert_eq!(
        result
            .observations
            .iter()
            .map(|row| &row.identity_id)
            .collect::<BTreeSet<_>>()
            .len(),
        1
    );
    assert!(
        result
            .pairs
            .iter()
            .any(|pair| matches!(pair.reason, PairReason::ExactNameFullAddressThreshold))
    );
    assert!(
        result
            .pairs
            .iter()
            .any(|pair| matches!(pair.reason, PairReason::ExactDonorName))
    );
}
#[test]
fn ambiguous_company_attachment_does_not_bridge() {
    let result = resolve(
        &input(vec![
            observation(
                1,
                Some("acme"),
                Some("1 road"),
                Some("Company"),
                Some("00000001"),
            ),
            observation(2, Some("acme"), Some("1 road"), None, None),
            observation(
                3,
                Some("acme"),
                Some("1 road"),
                Some("Company"),
                Some("00000002"),
            ),
        ]),
        0.9999,
    );
    assert_eq!(
        result.observations[0].identity_id.as_deref(),
        Some("source-reported:companies-house:00000001")
    );
    assert_eq!(
        result.observations[2].identity_id.as_deref(),
        Some("source-reported:companies-house:00000002")
    );
    assert!(
        result.observations[1]
            .identity_id
            .as_ref()
            .unwrap()
            .starts_with("run-local:")
    );
    assert!(
        result
            .pairs
            .iter()
            .all(|edge| edge.disposition == PairDisposition::Review)
    );
}
#[test]
fn explicit_person_kind_cannot_anchor_or_join_company() {
    let result = resolve(
        &input(vec![
            observation(
                1,
                Some("smith"),
                Some("1 road"),
                Some("Individual"),
                Some("00000001"),
            ),
            observation(
                2,
                Some("smith"),
                Some("1 road"),
                Some("Company"),
                Some("00000001"),
            ),
        ]),
        0.9999,
    );
    assert_ne!(
        result.observations[0].identity_id,
        result.observations[1].identity_id
    );
    assert_eq!(result.pairs[0].disposition, PairDisposition::Rejected);
}
#[test]
fn company_parser_preserves_conservative_source_policy() {
    for (raw, expected) in [
        (" sc012345 ", Some("SC012345")),
        ("527227", None),
        ("NI 016363", None),
        ("AAAAAAAA", None),
        ("00527227", Some("00527227")),
    ] {
        assert_eq!(
            company(&observation(1, None, None, None, Some(raw))).as_deref(),
            expected
        );
    }
    let result = resolve(
        &input(vec![
            observation(1, None, None, None, Some("00527227")),
            observation(2, None, None, None, None),
        ]),
        0.9999,
    );
    assert!(result.observations[0].identity_id.is_some());
    assert!(result.observations[1].identity_id.is_none());
}
#[test]
fn reordered_input_produces_identical_results() {
    let observations = vec![
        observation(1, Some("charity"), Some("1 road"), Some("Charity"), None),
        observation(2, Some("charity"), Some("1 road"), Some("Charity"), None),
    ];
    let mut reverse = observations.clone();
    reverse.reverse();
    let a = resolve(&input(observations), 0.9999);
    let b = resolve(&input(reverse), 0.9999);
    assert_eq!(
        serde_json::to_value(a.observations).unwrap(),
        serde_json::to_value(b.observations).unwrap()
    );
    assert_eq!(
        serde_json::to_value(a.pairs).unwrap(),
        serde_json::to_value(b.pairs).unwrap()
    );
}
fn payment(id: u32, parent: Option<u32>, flag: Option<bool>) -> Payment {
    Payment {
        funding_entry_id: serde_json::from_value(serde_json::json!(format!(
            "00000000-0000-0000-0000-000000000001/{id}/1/funding/0"
        )))
        .unwrap(),
        member_id: "00000000-0000-0000-0000-000000000001".into(),
        parliament_member_id: 1,
        declaration_id: id,
        register_id: 1,
        funding_ordinal: 0,
        parent_declaration_id: parent,
        is_ultimate_payer_different: flag,
        donor_funder_id: None,
        payer_funder_id: None,
        ultimate_payer_funder_id: None,
    }
}
#[test]
fn parent_false_selects_root_payer_and_withholding_blocks_fallback() {
    let mut payer = observation(1, Some("guardian"), None, None, None);
    payer.role = FunderRole::Payer;
    payer.funder_id = serde_json::from_value(serde_json::json!(
        payer.funder_id.as_str().replace("donor", "payer")
    ))
    .unwrap();
    let child = payment(2, Some(1), Some(false));
    let input =
        ResolutionInput::new(vec![payer.clone()], vec![child.clone()], BTreeMap::new()).unwrap();
    match attribute(&input, &child).decision {
        AttributionDecision::Selected {
            selected_funder_id, ..
        } => assert_eq!(selected_funder_id, payer.funder_id),
        AttributionDecision::Unavailable { .. } => panic!("parent payer not selected"),
    }
    let mut ultimate = observation(2, None, None, None, None);
    ultimate.role = FunderRole::UltimatePayer;
    ultimate.source_scope = "funding_entry".into();
    ultimate.funding_entry_id = Some(child.funding_entry_id.clone());
    ultimate.funding_ordinal = Some(0);
    ultimate.funder_id = serde_json::from_value(serde_json::json!(format!(
        "{}/ultimate_payer",
        child.funding_entry_id.as_str()
    )))
    .unwrap();
    let mut child = child;
    child.ultimate_payer_funder_id = Some(ultimate.funder_id.clone());
    let input =
        ResolutionInput::new(vec![payer, ultimate], vec![child.clone()], BTreeMap::new()).unwrap();
    assert!(matches!(
        attribute(&input, &child).decision,
        AttributionDecision::Unavailable { .. }
    ));
}
#[test]
fn malformed_score_and_duplicate_input_are_rejected() {
    let observations = vec![
        observation(1, Some("a"), None, None, None),
        observation(2, Some("b"), None, None, None),
    ];
    assert!(
        ResolutionInput::new(
            vec![observations[0].clone(), observations[0].clone()],
            vec![],
            BTreeMap::new()
        )
        .is_err()
    );
    let input = input(observations);
    let scoring = input.scoring_input(1);
    for probability in [f64::NAN, 1.1, -0.1] {
        assert!(
            ScoredPairs::checked(
                vec![ScoredPair {
                    left: scoring.rows[0].key.clone(),
                    right: scoring.rows[1].key.clone(),
                    probability,
                    name_level: 2,
                    address_level: 1
                }],
                serde_json::json!({}),
                &scoring
            )
            .is_err()
        );
    }
}

#[test]
fn sparse_addresses_never_support_automatic_links() {
    for address in [
        "london",
        "sw1a 1aa",
        "london sw1a 1aa",
        "1 sw1a 1aa",
        "1 london",
    ] {
        let result = resolve(
            &input(vec![
                payer(observation(
                    1,
                    Some("john smith"),
                    Some(address),
                    None,
                    None,
                )),
                payer(observation(
                    2,
                    Some("john smith"),
                    Some(address),
                    None,
                    None,
                )),
            ]),
            0.9999,
        );
        assert_ne!(
            result.observations[0].identity_id,
            result.observations[1].identity_id
        );
        assert!(
            result
                .pairs
                .iter()
                .all(|edge| edge.disposition == PairDisposition::Review)
        );
    }
}

#[test]
fn parent_cycles_and_true_or_absent_flags_never_inherit_payer() {
    let mut payer = observation(1, Some("guardian"), None, None, None);
    payer.role = FunderRole::Payer;
    payer.funder_id = serde_json::from_value(serde_json::json!(
        payer.funder_id.as_str().replace("donor", "payer")
    ))
    .unwrap();
    for flag in [None, Some(true)] {
        let child = payment(2, Some(1), flag);
        let input = ResolutionInput::new(vec![payer.clone()], vec![child.clone()], BTreeMap::new())
            .unwrap();
        assert!(matches!(
            attribute(&input, &child).decision,
            AttributionDecision::Unavailable { .. }
        ));
    }
    let parent = payment(1, Some(2), None);
    let child = payment(2, Some(1), Some(false));
    let input =
        ResolutionInput::new(vec![payer], vec![parent, child.clone()], BTreeMap::new()).unwrap();
    assert!(matches!(
        attribute(&input, &child).decision,
        AttributionDecision::Unavailable {
            unavailable_reason: UnavailableReason::ParentCycle
        }
    ));
}

#[test]
fn duplicate_occurrences_remain_separate_when_name_identity_links() {
    let mut first = payment(1, None, None);
    let mut second = first.clone();
    second.funding_ordinal = 1;
    second.funding_entry_id = serde_json::from_value(serde_json::json!(format!(
        "{}/{}/{}/funding/1",
        second.member_id, second.declaration_id, second.register_id
    )))
    .unwrap();
    let mut observations = Vec::new();
    for occurrence in [&mut first, &mut second] {
        let mut donor = observation(1, Some("john smith"), None, Some("Individual"), None);
        donor.source_scope = "funding_entry".into();
        donor.funding_ordinal = Some(occurrence.funding_ordinal);
        donor.funding_entry_id = Some(occurrence.funding_entry_id.clone());
        donor.funder_id = serde_json::from_value(serde_json::json!(format!(
            "{}/donor",
            occurrence.funding_entry_id.as_str()
        )))
        .unwrap();
        occurrence.donor_funder_id = Some(donor.funder_id.clone());
        observations.push(donor);
    }
    let input = ResolutionInput::new(observations, vec![first, second], BTreeMap::new()).unwrap();
    let result = resolve(&input, 0.9999);
    assert_eq!(result.attributions.len(), 2);
    assert_eq!(result.observations.len(), 2);
    assert_eq!(result.pairs.len(), 1);
    assert_eq!(
        result.observations[0].identity_id,
        result.observations[1].identity_id
    );
    assert_eq!(result.pairs[0].disposition, PairDisposition::Accepted);
}

#[test]
fn parent_payer_from_another_member_is_not_selected() {
    let mut payer = observation(1, Some("guardian"), None, None, None);
    payer.role = FunderRole::Payer;
    payer.member_id = uuid::Uuid::from_u128(2).to_string();
    payer.funder_id = serde_json::from_value(serde_json::json!(format!(
        "{}/1/1/declaration/payer",
        payer.member_id
    )))
    .unwrap();
    let child = payment(2, Some(1), Some(false));
    let input = ResolutionInput::new(vec![payer], vec![child.clone()], BTreeMap::new()).unwrap();
    assert!(matches!(
        attribute(&input, &child).decision,
        AttributionDecision::Unavailable {
            unavailable_reason: UnavailableReason::ParentEvidenceUnavailable
        }
    ));
}

#[test]
fn multiple_parent_root_payers_remain_ambiguous() {
    let mut observations = Vec::new();
    let mut payments = Vec::new();
    for ordinal in 0..2 {
        let mut parent = payment(1, None, None);
        parent.funding_ordinal = ordinal;
        parent.funding_entry_id = serde_json::from_value(serde_json::json!(format!(
            "{}/1/1/funding/{ordinal}",
            parent.member_id
        )))
        .unwrap();
        let mut payer = observation(1, Some("parent payer"), None, None, None);
        payer.role = FunderRole::Payer;
        payer.source_scope = "funding_entry".into();
        payer.funding_entry_id = Some(parent.funding_entry_id.clone());
        payer.funding_ordinal = Some(ordinal);
        payer.funder_id = serde_json::from_value(serde_json::json!(format!(
            "{}/payer",
            parent.funding_entry_id.as_str()
        )))
        .unwrap();
        parent.payer_funder_id = Some(payer.funder_id.clone());
        observations.push(payer);
        payments.push(parent);
    }
    let child = payment(2, Some(1), Some(false));
    payments.push(child.clone());
    let input = ResolutionInput::new(observations, payments, BTreeMap::new()).unwrap();
    assert!(matches!(
        attribute(&input, &child).decision,
        AttributionDecision::Unavailable {
            unavailable_reason: UnavailableReason::ParentPayerAmbiguous
        }
    ));
}

#[test]
fn indirect_links_cannot_join_distinct_company_components() {
    let observations = (1..=4)
        .map(|id| {
            let company = match id {
                1 => Some("00000001"),
                4 => Some("00000002"),
                _ => None,
            };
            observation(
                id,
                Some("shared name"),
                Some("10 example road"),
                None,
                company,
            )
        })
        .collect::<Vec<_>>();
    let input = input(observations);
    let rows = input
        .observations
        .values()
        .enumerate()
        .map(|(index, observation)| input::ComparisonRow {
            key: format!("profile-{index}"),
            name: observation.name_normalized.clone().unwrap(),
            address: observation.address_normalized.clone(),
            blocking_keys: Vec::new(),
            needs_resolution: company(observation).is_none(),
        })
        .collect::<Vec<_>>();
    let members = rows
        .iter()
        .zip(input.observations.keys())
        .map(|(row, id)| (row.key.clone(), vec![id.clone()]))
        .collect();
    let scoring = ScoringInput {
        version: 2,
        candidate_budget: 10,
        rows,
        members,
    };
    let pairs = scoring
        .rows
        .windows(2)
        .map(|pair| ScoredPair {
            left: pair[0].key.clone(),
            right: pair[1].key.clone(),
            probability: 0.9999,
            name_level: 2,
            address_level: 1,
        })
        .collect();
    let scored =
        ScoredPairs::checked(pairs, serde_json::json!({"calibrated": false}), &scoring).unwrap();
    let result = ResolvedDeclarations::from_scored(&input, &scoring, scored, "test-run").unwrap();
    assert_eq!(
        result.observations[0].identity_id.as_deref(),
        Some("source-reported:companies-house:00000001")
    );
    assert_eq!(
        result.observations[3].identity_id.as_deref(),
        Some("source-reported:companies-house:00000002")
    );
    assert_eq!(
        result
            .pairs
            .iter()
            .filter(|pair| pair.disposition == PairDisposition::Accepted)
            .count(),
        1
    );
    assert_eq!(
        result
            .pairs
            .iter()
            .filter(|pair| pair.disposition == PairDisposition::Review
                && matches!(pair.reason, PairReason::CompetingCompanyAnchors))
            .count(),
        2
    );
}

fn resolve_edges(
    observations: Vec<Observation>,
    links: &[(usize, usize, f64)],
) -> ResolvedDeclarations {
    let input = input(observations);
    let ids = input.observations.keys().cloned().collect::<Vec<_>>();
    let scoring = ScoringInput {
        version: 2,
        candidate_budget: 100,
        rows: ids
            .iter()
            .enumerate()
            .map(|(index, id)| input::ComparisonRow {
                key: format!("profile-{index}"),
                name: input.observations[id].name_normalized.clone().unwrap(),
                address: input.observations[id].address_normalized.clone(),
                blocking_keys: Vec::new(),
                needs_resolution: company(&input.observations[id]).is_none(),
            })
            .collect(),
        members: ids
            .iter()
            .enumerate()
            .map(|(index, id)| (format!("profile-{index}"), vec![id.clone()]))
            .collect(),
    };
    let pairs = links
        .iter()
        .map(|(a, b, probability)| ScoredPair {
            left: format!("profile-{a}"),
            right: format!("profile-{b}"),
            probability: *probability,
            name_level: 2,
            address_level: 1,
        })
        .collect();
    let scored =
        ScoredPairs::checked(pairs, serde_json::json!({"calibrated": false}), &scoring).unwrap();
    ResolvedDeclarations::from_scored(&input, &scoring, scored, "test-run").unwrap()
}
#[test]
fn indirect_competing_anchors_abstain_under_reordered_ids() {
    for ids in [[1, 2, 3, 4], [4, 1, 3, 2], [2, 4, 1, 3]] {
        let observations = ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                observation(
                    *id,
                    Some("acme"),
                    None,
                    Some("Company"),
                    match index {
                        0 => Some("00000001"),
                        3 => Some("00000002"),
                        _ => None,
                    },
                )
            })
            .collect::<Vec<_>>();
        let position = |role: usize| ids.iter().filter(|id| **id < ids[role]).count();
        let links = [(0, 1), (1, 2), (2, 3)].map(|(a, b)| {
            let (a, b) = (position(a), position(b));
            (a.min(b), a.max(b), 0.01)
        });
        let result = resolve_edges(observations, &links);
        let unanchored = result
            .observations
            .iter()
            .filter(|row| matches!(row.identity_basis, IdentityBasis::DonorNameLink))
            .collect::<Vec<_>>();
        assert_eq!(unanchored.len(), 2);
        assert_eq!(unanchored[0].identity_id, unanchored[1].identity_id);
        assert_eq!(
            result
                .pairs
                .iter()
                .filter(|pair| matches!(pair.reason, PairReason::CompetingCompanyAnchors))
                .count(),
            2
        );
        assert_eq!(
            result
                .pairs
                .iter()
                .filter(|pair| pair.disposition == PairDisposition::Accepted)
                .count(),
            1
        );
    }
}
#[test]
fn redundant_donor_name_edge_does_not_downgrade_statistical_component() {
    let result = resolve_edges(
        (1..=3)
            .map(|id| observation(id, Some("john smith"), Some("1 road"), None, None))
            .collect(),
        &[(0, 1, 0.9999), (1, 2, 0.9999), (0, 2, 0.01)],
    );
    assert!(
        result
            .observations
            .iter()
            .all(|row| matches!(row.identity_basis, IdentityBasis::StatisticalLink))
    );
    assert!(
        result
            .pairs
            .iter()
            .any(|pair| matches!(pair.reason, PairReason::ExactDonorName))
    );
}
#[test]
fn unknown_donor_bridge_cannot_mix_person_and_organisation_components() {
    let result = resolve_edges(
        vec![
            observation(1, Some("shared"), None, Some("Individual"), None),
            observation(2, Some("shared"), None, None, None),
            observation(3, Some("shared"), None, Some("Company"), None),
        ],
        &[(0, 1, 0.01), (1, 2, 0.01)],
    );
    assert_ne!(
        result.observations[0].identity_id,
        result.observations[2].identity_id
    );
    assert!(
        result
            .pairs
            .iter()
            .any(|pair| matches!(pair.reason, PairReason::ComponentIdentityConflict))
    );
}

#[test]
fn other_roles_keep_probability_and_address_requirements() {
    for (address, probability, disposition) in [
        (Some("1 road"), 0.01, PairDisposition::Review),
        (Some("2 road"), 0.9999, PairDisposition::Rejected),
        (Some("1 road"), 0.9999, PairDisposition::Accepted),
    ] {
        let result = resolve(
            &input(vec![
                observation(1, Some("john smith"), Some("1 road"), None, None),
                payer(observation(2, Some("john smith"), address, None, None)),
            ]),
            probability,
        );
        assert_eq!(result.pairs[0].disposition, disposition);
    }
}
#[test]
fn donor_name_attachment_preserves_seeded_company_source_basis() {
    let result = resolve(
        &input(vec![
            observation(1, Some("acme"), None, Some("Company"), Some("00000001")),
            observation(
                2,
                Some("another captured name"),
                None,
                Some("Company"),
                Some("00000001"),
            ),
            observation(3, Some("acme"), None, Some("Company"), None),
        ]),
        0.01,
    );
    assert!(
        result.observations[..2]
            .iter()
            .all(|row| matches!(row.identity_basis, IdentityBasis::SourceReportedCompany))
    );
    assert!(matches!(
        result.observations[2].identity_basis,
        IdentityBasis::DonorNameLink
    ));
    assert!(
        result
            .observations
            .iter()
            .all(|row| row.identity_id.as_deref()
                == Some("source-reported:companies-house:00000001"))
    );
}
