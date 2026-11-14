//! Current-term rankings retain exact totals and distinct entity identities.
use super::{
    entity_details::{DeclaredGbpPeriodTotal, MemberDeclaration},
    funder::{FunderId, FunderKind},
    parliament_member::MemberId,
};

/// Minimal member identity in a ranking.
#[derive(Clone, Debug)]
pub struct RankedMember {
    /// Parliament identity.
    pub id: MemberId,
    /// Latest stored name.
    pub name: String,
    /// Latest party label.
    pub party_name: String,
    /// Latest constituency or membership description.
    pub membership_from: String,
}
/// Canonical funder identity and classification.
#[derive(Clone, Debug)]
pub struct RankedFunder {
    /// Canonical alias-group identity.
    pub id: FunderId,
    /// Preferred group name.
    pub name: String,
    /// Canonical root classification.
    pub kind: FunderKind,
}
/// Recipient's latest party identity.
#[derive(Clone, Debug)]
pub struct RankedParty {
    /// Stored party identity.
    pub id: i32,
    /// Deterministic label across recipients.
    pub name: String,
}
/// A ranked identity and its backend-owned financial total.
#[derive(Debug)]
pub struct Ranking<I> {
    /// Entity being ranked.
    pub identity: I,
    /// Exact declared GBP total and coverage.
    pub total: DeclaredGbpPeriodTotal,
}
/// Four complete ordered boards from one snapshot for the selected period.
#[derive(Debug)]
pub struct Rankings {
    /// Scope shared by all boards.
    pub period: super::reporting_period::ReportingPeriod,
    /// Inclusive current-term boundary, independent of the selected period.
    pub current_term_start: chrono::DateTime<chrono::Utc>,
    /// All-category member totals.
    pub members: Vec<Ranking<RankedMember>>,
    /// Donation/support totals from canonical individual funders.
    pub individuals: Vec<Ranking<RankedFunder>>,
    /// Donation/support totals from all canonical non-individual funders, including unclassified.
    pub companies: Vec<Ranking<RankedFunder>>,
    /// Donation/support declared by recipient MPs' latest party.
    pub parties: Vec<Ranking<RankedParty>>,
}
/// One declaration with canonical funder metadata keyed by entry references.
pub struct RankingDeclaration {
    /// Recipient member.
    pub member: RankedMember,
    /// Recipient's latest party.
    pub party: RankedParty,
    /// Each stored funding occurrence, including unnamed entries.
    pub declaration: MemberDeclaration,
    /// Canonical funders referenced by the declaration entries.
    pub funders: std::collections::BTreeMap<uuid::Uuid, RankedFunder>,
}
