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
        name_normalized: name.map(str::to_owned),
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
#[test]
fn noncompanies_share_only_exact_name_full_address() {
    let result = resolve(
        &input(vec![
            observation(
                1,
                Some("john smith"),
                Some("1 road"),
                Some("Individual"),
                None,
            ),
            observation(
                2,
                Some("john smith"),
                Some("1 road"),
                Some("Individual"),
                None,
            ),
            observation(
                3,
                Some("john smith"),
                Some("2 road"),
                Some("Individual"),
                None,
            ),
        ]),
        0.9999,
    );
    assert_eq!(
        result.observations[0].identity_id,
        result.observations[1].identity_id
    );
    assert_ne!(
        result.observations[0].identity_id,
        result.observations[2].identity_id
    );
    assert!(
        result
            .pairs
            .iter()
            .any(|edge| edge.disposition == PairDisposition::Rejected)
    );
}
#[test]
fn fuzzy_and_name_only_remain_review_and_singletons() {
    for address in [None, Some("1 road")] {
        let result = resolve(
            &input(vec![
                observation(1, Some("john smith"), address, None, None),
                observation(
                    2,
                    Some(if address.is_some() {
                        "jon smith"
                    } else {
                        "john smith"
                    }),
                    address,
                    None,
                    None,
                ),
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
                observation(1, Some("john smith"), Some(address), None, None),
                observation(2, Some("john smith"), Some(address), None, None),
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
fn duplicate_occurrences_remain_separate_and_profile_grouping_does_not_merge_names() {
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
    assert_ne!(
        result.observations[0].identity_id,
        result.observations[1].identity_id
    );
    assert_eq!(result.pairs[0].disposition, PairDisposition::Review);
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
            needs_resolution: company(observation).is_none(),
        })
        .collect::<Vec<_>>();
    let members = rows
        .iter()
        .zip(input.observations.keys())
        .map(|(row, id)| (row.key.clone(), vec![id.clone()]))
        .collect();
    let scoring = ScoringInput {
        version: 1,
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
        2
    );
    assert!(
        result
            .pairs
            .iter()
            .any(|pair| pair.disposition == PairDisposition::Rejected
                && matches!(pair.reason, PairReason::ComponentIdentityConflict))
    );
}
