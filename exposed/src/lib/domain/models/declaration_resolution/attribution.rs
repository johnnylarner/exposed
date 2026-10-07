//! One reporting selection per funding occurrence, independent of identity links.
use super::{
    AttributionBasis, AttributionDecision, AttributionIssue, BTreeSet, FunderObservationId,
    FunderRole, Payment, PaymentAttribution, ResolutionInput, UnavailableReason,
};
use super::{identity::company, input::is_root};

pub(super) fn attribute(input: &ResolutionInput, payment: &Payment) -> PaymentAttribution {
    let mut issues = Vec::new();
    if let Some(id) = &payment.ultimate_payer_funder_id {
        if payment.is_ultimate_payer_different == Some(false) {
            issues.push(AttributionIssue::ExplicitUltimatePayerWithParentPayerSameFlag);
        }
        let decision = local_selection(
            input,
            id,
            AttributionBasis::ExplicitUltimatePayer,
            UnavailableReason::ExplicitUltimatePayerUnavailable,
        );
        return PaymentAttribution {
            funding_entry_id: payment.funding_entry_id.clone(),
            decision,
            issues,
        };
    }
    let decision = match payment.is_ultimate_payer_different {
        Some(true) => unavailable(UnavailableReason::DifferentUltimatePayerUnnamed),
        Some(false) => parent_selection(input, payment),
        None => match (&payment.donor_funder_id, &payment.payer_funder_id) {
            (Some(id), _) => local_selection(
                input,
                id,
                AttributionBasis::Donor,
                UnavailableReason::DonorUnavailable,
            ),
            (None, Some(id)) => local_selection(
                input,
                id,
                AttributionBasis::Payer,
                UnavailableReason::PayerUnavailable,
            ),
            (None, None) => unavailable(UnavailableReason::NoSupportedAttribution),
        },
    };
    PaymentAttribution {
        funding_entry_id: payment.funding_entry_id.clone(),
        decision,
        issues,
    }
}
fn usable(input: &ResolutionInput, id: &FunderObservationId) -> bool {
    let observation = &input.observations[id];
    observation.name_normalized.is_some() || company(observation).is_some()
}
const fn unavailable(reason: UnavailableReason) -> AttributionDecision {
    AttributionDecision::Unavailable {
        unavailable_reason: reason,
    }
}
fn local_selection(
    input: &ResolutionInput,
    id: &FunderObservationId,
    basis: AttributionBasis,
    reason: UnavailableReason,
) -> AttributionDecision {
    if usable(input, id) {
        AttributionDecision::Selected {
            selected_funder_id: id.clone(),
            attribution_basis: basis,
            selected_parent_declaration_id: None,
        }
    } else {
        unavailable(reason)
    }
}
fn parent_selection(input: &ResolutionInput, payment: &Payment) -> AttributionDecision {
    if parent_cycle(input, payment) {
        return unavailable(UnavailableReason::ParentCycle);
    }
    let Some(parent) = payment.parent_declaration_id else {
        return unavailable(UnavailableReason::ParentEvidenceUnavailable);
    };
    let candidates = input
        .observations
        .values()
        .filter(|observation| {
            observation.member_id == payment.member_id
                && observation.declaration_id == parent
                && observation.role == FunderRole::Payer
                && is_root(&observation.source_pointer)
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [payer] if usable(input, &payer.funder_id) => AttributionDecision::Selected {
            selected_funder_id: payer.funder_id.clone(),
            attribution_basis: AttributionBasis::ParentPayer,
            selected_parent_declaration_id: Some(parent),
        },
        [_] => unavailable(UnavailableReason::ParentPayerUnavailable),
        [] => unavailable(UnavailableReason::ParentEvidenceUnavailable),
        _ => unavailable(UnavailableReason::ParentPayerAmbiguous),
    }
}
fn parent_cycle(input: &ResolutionInput, payment: &Payment) -> bool {
    let mut seen = BTreeSet::new();
    let mut next = Some(payment.declaration_id);
    while let Some(id) = next {
        if !seen.insert(id) {
            return true;
        }
        next = input
            .payments
            .values()
            .find(|candidate| {
                candidate.member_id == payment.member_id && candidate.declaration_id == id
            })
            .and_then(|candidate| candidate.parent_declaration_id);
    }
    false
}
