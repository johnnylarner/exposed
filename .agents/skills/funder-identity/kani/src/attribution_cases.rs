use super::{DeclarationId, FundingEntryId, MemberId, Observation};

fn proof_payment(parent: bool) -> Payment {
    Payment {
        funding_entry_id: FundingEntryId(
            if parent {
                "1/1/1/funding/0"
            } else {
                "1/2/1/funding/0"
            }
            .into(),
        ),
        member_id: MemberId(1),
        declaration_id: DeclarationId(
            std::num::NonZeroU32::new(if parent { 1 } else { 2 }).unwrap(),
        ),
        register_id: 1,
        funding_ordinal: 0,
        parent_declaration_id: None,
        is_ultimate_payer_different: None,
        donor_funder_id: None,
        payer_funder_id: None,
        ultimate_payer_funder_id: None,
    }
}

fn proof_role(id: &str, payment: &Payment, role: FunderRole) -> Observation {
    let mut observation = super::observation(id);
    observation.member_id = payment.member_id;
    observation.declaration_id = payment.declaration_id;
    observation.role = role;
    observation.source_scope = "funding_entry".into();
    observation.funding_entry_id = Some(payment.funding_entry_id.clone());
    observation.funding_ordinal = Some(payment.funding_ordinal);
    observation
}

fn proof_input(observations: Vec<Observation>, payments: Vec<Payment>) -> ResolutionInput {
    let mut input = ResolutionInput {
        observations: Default::default(),
        payments: Default::default(),
        digests: Default::default(),
    };
    for observation in observations {
        input
            .observations
            .insert(observation.funder_id.clone(), observation);
    }
    for payment in payments {
        input
            .payments
            .insert(payment.funding_entry_id.clone(), payment);
    }
    input
}

fn check_parent_eligibility_and_flag_gate(
    same_member: bool,
    immediate_parent: bool,
    root: bool,
    payer_role: bool,
    flag: u8,
) {
    let mut child = proof_payment(false);
    child.parent_declaration_id = Some(proof_payment(true).declaration_id);
    child.is_ultimate_payer_different = [None, Some(false), Some(true)][usize::from(flag)];
    let mut parent = proof_payment(true);
    if !same_member {
        parent.member_id = MemberId(2);
        parent.funding_entry_id = FundingEntryId("2/1/1/funding/0".into());
    }
    if !immediate_parent {
        parent.declaration_id = DeclarationId(std::num::NonZeroU32::new(3).unwrap());
        parent.funding_entry_id = FundingEntryId(
            if same_member {
                "1/3/1/funding/0"
            } else {
                "2/3/1/funding/0"
            }
            .into(),
        );
    }
    let role = if payer_role {
        FunderRole::Payer
    } else {
        FunderRole::Donor
    };
    let mut candidate = proof_role("p", &parent, role);
    if !root {
        candidate.source_pointer = "/versions/0/fields/0/values/0".into();
    }
    if payer_role {
        parent.payer_funder_id = Some(candidate.funder_id.clone());
    } else {
        parent.donor_funder_id = Some(candidate.funder_id.clone());
    }
    let id = candidate.funder_id.clone();
    let input = proof_input(vec![candidate], vec![parent, child.clone()]);
    let decision = attribute(&input, &child).decision;
    let correct = match flag {
        0 => matches!(
            decision,
            AttributionDecision::Unavailable {
                unavailable_reason: UnavailableReason::NoSupportedAttribution
            }
        ),
        2 => matches!(
            decision,
            AttributionDecision::Unavailable {
                unavailable_reason: UnavailableReason::DifferentUltimatePayerUnnamed
            }
        ),
        _ if same_member && immediate_parent && root && payer_role => matches!(decision,
            AttributionDecision::Selected { selected_funder_id, attribution_basis: AttributionBasis::ParentPayer, selected_parent_declaration_id }
            if selected_funder_id == id && selected_parent_declaration_id == child.parent_declaration_id),
        _ => matches!(
            decision,
            AttributionDecision::Unavailable {
                unavailable_reason: UnavailableReason::ParentEvidenceUnavailable
            }
        ),
    };
    assert!(
        correct,
        "parent selection requires false flag and same-member immediate-parent root payer"
    );
}

fn check_explicit_ultimate_precedence_and_withholding(usable: bool, flag: u8) {
    let mut child = proof_payment(false);
    child.is_ultimate_payer_different = [None, Some(false), Some(true)][usize::from(flag)];
    child.parent_declaration_id = Some(proof_payment(true).declaration_id);
    let donor = proof_role("d", &child, FunderRole::Donor);
    let payer = proof_role("p", &child, FunderRole::Payer);
    let mut ultimate = proof_role("u", &child, FunderRole::UltimatePayer);
    if !usable {
        ultimate.name_raw = None;
        ultimate.name_normalized = None;
    }
    child.donor_funder_id = Some(donor.funder_id.clone());
    child.payer_funder_id = Some(payer.funder_id.clone());
    child.ultimate_payer_funder_id = Some(ultimate.funder_id.clone());
    let id = ultimate.funder_id.clone();
    let input = proof_input(vec![donor, payer, ultimate], vec![child.clone()]);
    let result = attribute(&input, &child);
    let correct = if usable {
        matches!(result.decision, AttributionDecision::Selected {
            selected_funder_id, attribution_basis: AttributionBasis::ExplicitUltimatePayer,
            selected_parent_declaration_id: None
        } if selected_funder_id == id)
    } else {
        matches!(
            result.decision,
            AttributionDecision::Unavailable {
                unavailable_reason: UnavailableReason::ExplicitUltimatePayerUnavailable
            }
        )
    };
    assert!(
        correct,
        "explicit ultimate payer wins and withheld ultimate never falls back"
    );
    assert!(
        result.issues.len() == usize::from(flag == 1),
        "explicit ultimate false flag records exactly one conflict issue"
    );
    if flag == 1 {
        assert!(
            matches!(
                result.issues[0],
                AttributionIssue::ExplicitUltimatePayerWithParentPayerSameFlag
            ),
            "ultimate conflict retains its specific issue"
        );
    }
}

fn check_absent_and_withheld_local_donor_have_distinct_results(
    donor_present: bool,
    donor_usable: bool,
) {
    let mut child = proof_payment(false);
    let payer = proof_role("p", &child, FunderRole::Payer);
    let mut donor = proof_role("d", &child, FunderRole::Donor);
    donor.donor_kind = Some("Company".into());
    if !donor_usable {
        donor.name_raw = None;
        donor.name_normalized = None;
    }
    child.payer_funder_id = Some(payer.funder_id.clone());
    let payer_id = payer.funder_id.clone();
    let donor_id = donor.funder_id.clone();
    let mut observations = vec![payer];
    if donor_present {
        child.donor_funder_id = Some(donor_id.clone());
        observations.push(donor);
    }
    let input = proof_input(observations, vec![child.clone()]);
    let correct = match (
        donor_present,
        donor_usable,
        attribute(&input, &child).decision,
    ) {
        (
            false,
            _,
            AttributionDecision::Selected {
                selected_funder_id,
                attribution_basis: AttributionBasis::Payer,
                selected_parent_declaration_id: None,
            },
        ) => selected_funder_id == payer_id,
        (
            true,
            true,
            AttributionDecision::Selected {
                selected_funder_id,
                attribution_basis: AttributionBasis::Donor,
                selected_parent_declaration_id: None,
            },
        ) => selected_funder_id == donor_id,
        (
            true,
            false,
            AttributionDecision::Unavailable {
                unavailable_reason: UnavailableReason::DonorUnavailable,
            },
        ) => true,
        _ => false,
    };
    assert!(
        correct,
        "absent donor permits payer but withheld donor blocks payer fallback"
    );
}

fn check_parent_ambiguity_withholding_and_cycles_block_fallback(
    cycle: bool,
    second_candidate: bool,
    usable: bool,
) {
    let mut child = proof_payment(false);
    let mut parent = proof_payment(true);
    child.parent_declaration_id = Some(parent.declaration_id);
    child.is_ultimate_payer_different = Some(false);
    if cycle {
        parent.parent_declaration_id = Some(child.declaration_id);
    }
    let mut donor = proof_role("d", &child, FunderRole::Donor);
    donor.donor_kind = Some("Company".into());
    donor.donor_company_number = Some("00000001".into());
    child.donor_funder_id = Some(donor.funder_id.clone());
    let mut payer = proof_role("p", &parent, FunderRole::Payer);
    if !usable {
        payer.name_raw = None;
        payer.name_normalized = None;
    }
    let id = payer.funder_id.clone();
    parent.payer_funder_id = Some(id.clone());
    let mut observations = vec![donor, payer];
    let mut payments = vec![child.clone(), parent.clone()];
    if second_candidate {
        parent.funding_entry_id = FundingEntryId("1/1/1/funding/1".into());
        parent.funding_ordinal = 1;
        let other = proof_role("q", &parent, FunderRole::Payer);
        parent.payer_funder_id = Some(other.funder_id.clone());
        observations.push(other);
        payments.push(parent);
    }
    let input = proof_input(observations, payments);
    let decision = attribute(&input, &child).decision;
    let correct = if cycle {
        matches!(
            decision,
            AttributionDecision::Unavailable {
                unavailable_reason: UnavailableReason::ParentCycle
            }
        )
    } else if second_candidate {
        matches!(
            decision,
            AttributionDecision::Unavailable {
                unavailable_reason: UnavailableReason::ParentPayerAmbiguous
            }
        )
    } else if !usable {
        matches!(
            decision,
            AttributionDecision::Unavailable {
                unavailable_reason: UnavailableReason::ParentPayerUnavailable
            }
        )
    } else {
        matches!(decision, AttributionDecision::Selected { selected_funder_id, attribution_basis: AttributionBasis::ParentPayer, selected_parent_declaration_id }
            if selected_funder_id == id && selected_parent_declaration_id == child.parent_declaration_id)
    };
    assert!(
        correct,
        "parent cycles outrank ambiguity and unusable parents never fall back to donor"
    );
}
