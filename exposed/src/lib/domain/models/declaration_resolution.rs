//! Identity decisions and payment attribution from checked cleaned evidence.

use super::{
    declaration_cleaning::{AddressMatchQuality, FunderObservationId, FunderRole, FundingEntryId},
    entity_ingestion::EntityIngestionError,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use strum::AsRefStr;

pub(crate) const POLICY_VERSION: &str = "funder-resolution-v3";
pub(crate) const AUTOMATIC_THRESHOLD: f64 = 0.999;

mod attribution;
mod identity;
fn invalid(message: impl Into<String>) -> EntityIngestionError {
    EntityIngestionError::DataError(message.into())
}

mod input;
use attribution::attribute;
#[cfg(test)]
use identity::company;
pub(crate) use input::{ComparisonRow, Observation, Payment, ScoredPair};
pub use input::{ResolutionInput, ScoredPairs, ScoringInput};

/// Evidence basis of an assigned identity, independent of reporting attribution.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityBasis {
    /// An accepted captured company number, without registry verification.
    SourceReportedCompany,
    /// An accepted exact name and full address score.
    StatisticalLink,
    /// Exact donor names are necessary to connect the identity.
    DonorNameLink,
    /// Cleaner-extracted aliases or organization evidence connect the identity.
    ExtractedNameLink,
    /// Typed trade-union family evidence connects the identity.
    TradeUnionFamilyLink,
    /// Statistical and exact donor-name links support the identity.
    StatisticalAndDonorNameLink,
    /// Statistical links combine with extracted aliases or typed domain evidence.
    StatisticalAndSupportingNameLink,
    /// A separate local identity without evidence of a cross-observation match.
    ProvisionalSingleton,
    /// No usable name or eligible captured identifier.
    Unresolved,
}
/// One outcome for each input source-role observation.
#[derive(Serialize, Deserialize)]
pub struct ObservationResolution {
    pub(crate) funder_id: FunderObservationId,
    pub(crate) identity_id: Option<String>,
    pub(crate) identity_basis: IdentityBasis,
}
/// Policy disposition of one statistical candidate edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairDisposition {
    /// The edge passed its evidence rule and component constraints.
    Accepted,
    /// The edge remains a candidate for human review.
    Review,
    /// Retained evidence contradicts automatic merging.
    Rejected,
}
/// Evidence reason for a candidate disposition.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairReason {
    /// The initial exact name and numbered-street rule is not satisfied.
    InsufficientExactEvidence,
    /// Explicit person and organisation kinds disagree.
    PersonOrganisationConflict,
    /// Two usable numbered-street addresses disagree.
    FullAddressDisagreement,
    /// An eligible unanchored group reaches more than one company anchor.
    CompetingCompanyAnchors,
    /// A union would mix company numbers or incompatible entity kinds.
    ComponentIdentityConflict,
    /// Exact names, full addresses, threshold, and component constraints agree.
    ExactNameFullAddressThreshold,
    /// Both donors have the same usable normalized name.
    ExactDonorName,
    /// Cleaner-extracted aliases or matching organization evidence support the link.
    ExtractedNameEvidence,
    /// Both observations are explicitly typed trade unions with the distinctive Unite root.
    TradeUnionFamily,
}
/// A scored edge retained for inspection, including rejected evidence.
#[derive(Serialize, Deserialize)]
pub struct PairDecision {
    pub(crate) left_funder_id: FunderObservationId,
    pub(crate) right_funder_id: FunderObservationId,
    pub(crate) probability: f64,
    pub(crate) name_level: i32,
    pub(crate) address_level: i32,
    pub(crate) disposition: PairDisposition,
    pub(crate) reason: PairReason,
}
/// Source role used for one payment's reporting decision.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(test, derive(strum::EnumIter))]
pub enum AttributionBasis {
    /// An explicitly named ultimate payer.
    ExplicitUltimatePayer,
    /// The unique root payer of the immediate linked parent.
    ParentPayer,
    /// The local donor when no ultimate-payer flag is supplied.
    Donor,
    /// The local payer when no donor or ultimate-payer flag is supplied.
    Payer,
}
/// Evidence that prevents supported reporting attribution.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(test, derive(strum::EnumIter))]
pub enum UnavailableReason {
    /// An explicit ultimate role has no usable name.
    ExplicitUltimatePayerUnavailable,
    /// A different ultimate payer is declared but unnamed.
    DifferentUltimatePayerUnnamed,
    /// Loaded parent links contain a cycle.
    ParentCycle,
    /// The unique parent payer is withheld or unnamed.
    ParentPayerUnavailable,
    /// No same-member immediate-parent root payer is retained.
    ParentEvidenceUnavailable,
    /// More than one parent root payer is retained.
    ParentPayerAmbiguous,
    /// An explicit donor lacks usable evidence.
    DonorUnavailable,
    /// An explicit payer lacks usable evidence.
    PayerUnavailable,
    /// No source role supports a selection.
    NoSupportedAttribution,
}
/// A disagreement between source fields, without an entity-distinction claim.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionIssue {
    /// A named ultimate role occurs with the same-parent-payer flag.
    ExplicitUltimatePayerWithParentPayerSameFlag,
}
/// A selected source role, or an explicit reporting failure.
#[derive(Serialize, Deserialize)]
#[serde(tag = "attribution_status", rename_all = "snake_case")]
pub enum AttributionDecision {
    /// One supported observation and its source basis.
    Selected {
        /// Observation retained in the cleaned evidence.
        selected_funder_id: FunderObservationId,
        /// Source role used by the attribution policy.
        attribution_basis: AttributionBasis,
        /// Immediate parent only when a parent payer was selected.
        selected_parent_declaration_id: Option<u32>,
    },
    /// An explicit reason no source observation could be selected.
    Unavailable {
        /// Evidence failure retained for reporting.
        unavailable_reason: UnavailableReason,
    },
}
/// Exactly one attribution row for one funding occurrence.
#[derive(Serialize, Deserialize)]
pub struct PaymentAttribution {
    pub(crate) funding_entry_id: FundingEntryId,
    #[serde(flatten)]
    pub(crate) decision: AttributionDecision,
    pub(crate) issues: Vec<AttributionIssue>,
}
/// Complete immutable identity, candidate, and payment results.
pub struct ResolvedDeclarations {
    pub(crate) observations: Vec<ObservationResolution>,
    pub(crate) attributions: Vec<PaymentAttribution>,
    pub(crate) pairs: Vec<PairDecision>,
    pub(crate) manifest: serde_json::Value,
}
/// Counts published by one successful resolution operation.
#[derive(Clone, Debug)]
pub struct DeclarationResolutionSummary {
    /// Source observations, including unresolved names.
    pub observations: usize,
    /// Original funding occurrences, counted once.
    pub payments: usize,
    /// Statistical edges retained in the output.
    pub pairs: usize,
}

impl ResolvedDeclarations {
    /// Applies conservative identity links and the independent payment-role policy.
    ///
    /// # Errors
    /// Refuses expansion of repeated profiles beyond the configured candidate budget.
    pub fn from_scored(
        input: &ResolutionInput,
        scoring: &ScoringInput,
        scored: ScoredPairs,
        run_key: &str,
    ) -> Result<Self, EntityIngestionError> {
        let (observations, pairs) = identity::resolve(input, scoring, scored.pairs, run_key)?;
        let attributions = input
            .payments
            .values()
            .map(|payment| attribute(input, payment))
            .collect::<Vec<_>>();
        let manifest = serde_json::json!({ "schema_version":1, "rust_package_version":env!("CARGO_PKG_VERSION"), "policy_version":POLICY_VERSION, "automatic_threshold":AUTOMATIC_THRESHOLD,
            "input_sha256":input.digests, "model":scored.model, "scoring_profiles":scoring.rows.len(), "observations":observations.len(), "payments":attributions.len(),
            "pair_decisions":pairs.len(), "accepted_pairs":pairs.iter().filter(|p|p.disposition==PairDisposition::Accepted).count(),
            "review_pairs":pairs.iter().filter(|p|p.disposition==PairDisposition::Review).count(), "candidate_budget":scoring.candidate_budget,
            "identity_scope":"run-local IDs use ingestion key and lexicographically first component member; changed membership can change IDs; cross-run reconciliation unsupported" });
        Ok(Self {
            observations,
            attributions,
            pairs,
            manifest,
        })
    }

    /// Row counts of the complete result.
    #[must_use]
    pub const fn summary(&self) -> DeclarationResolutionSummary {
        DeclarationResolutionSummary {
            observations: self.observations.len(),
            payments: self.attributions.len(),
            pairs: self.pairs.len(),
        }
    }
}

#[cfg(test)]
mod tests;
