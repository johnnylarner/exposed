//! Stored profiles, declaration occurrences, and exact currency summaries.

use std::str::FromStr;

use bigdecimal::BigDecimal;
use thiserror::Error;

use crate::domain::repositories::entity_details::EntityDetailsRepoError;
use chrono::{DateTime, Utc};

use super::{
    funder::{Funder, FunderId},
    parliament_member::{MemberId, MemberIdError},
};

/// Entity detail lookup failure.
#[derive(Debug, Error)]
pub enum EntityDetailsError {
    /// Member identity could not be parsed.
    #[error("Invalid MP ID")]
    InvalidMemberId(#[source] MemberIdError),
    /// Funder identity could not be parsed.
    #[error("Invalid funder ID")]
    InvalidFunderId(#[source] uuid::Error),
    /// No stored identity matches the request.
    #[error("entity not found")]
    NotFound,
    /// Private database diagnostics.
    #[error(transparent)]
    Unexpected(#[from] EntityDetailsRepoError),
}

/// Parsed member detail lookup.
#[derive(Clone, Copy, Debug)]
pub struct MemberDetailsRequest {
    id: MemberId,
}

impl MemberDetailsRequest {
    /// Parliament identity to look up.
    #[must_use]
    pub const fn id(self) -> MemberId {
        self.id
    }
}

impl FromStr for MemberDetailsRequest {
    type Err = EntityDetailsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse()
            .map(|id| Self { id })
            .map_err(EntityDetailsError::InvalidMemberId)
    }
}

/// Parsed funder detail lookup.
#[derive(Clone, Copy, Debug)]
pub struct FunderDetailsRequest {
    id: FunderId,
}

impl FunderDetailsRequest {
    /// Persistent funder identity to look up.
    #[must_use]
    pub const fn id(self) -> FunderId {
        self.id
    }
}

impl FromStr for FunderDetailsRequest {
    type Err = EntityDetailsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse()
            .map(|id| Self { id })
            .map_err(EntityDetailsError::InvalidFunderId)
    }
}

/// Latest stored member profile, not membership at the time of a payment.
#[derive(Clone, Debug)]
pub struct MemberProfile {
    /// Parliament identity.
    pub id: MemberId,
    /// Source name.
    pub name: String,
    /// Latest recorded party identity.
    pub party_id: i32,
    /// Latest recorded party name.
    pub party_name: String,
    /// Latest membership description, including constituencies and Lords memberships.
    pub membership_from: String,
    /// Current Commons membership.
    pub is_current_commons: bool,
}

/// A named funder on a declaration.
#[derive(Clone, Debug)]
pub struct FunderReference {
    /// Persistent identity.
    pub id: FunderId,
    /// Exact source name.
    pub name: String,
}

/// One source funding occurrence, including incomplete values.
#[derive(Clone, Debug)]
pub struct DeclaredFunding {
    /// Named funder when available.
    pub funder: Option<FunderReference>,
    /// Exact source amount.
    pub amount: Option<BigDecimal>,
    /// Normalized source currency, absent when unknown.
    pub currency: Option<String>,
    /// Source payment classification.
    pub payment_type: Option<String>,
}

/// A declaration with all of its funding occurrences.
#[derive(Clone, Debug)]
pub struct MemberDeclaration {
    /// Parliament declaration identity.
    pub source_id: i32,
    /// Canonical source category classification.
    pub category: DeclarationCategory,
    /// Source category.
    pub category_name: String,
    /// Registration date when available.
    pub registered_at: Option<DateTime<Utc>>,
    /// Funding occurrences, preserving duplicates.
    pub entries: Vec<DeclaredFunding>,
}

/// Canonical classification preserves unrecognized source categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DeclarationCategory {
    /// Donations and other support, including loans.
    DonationSupport,
    /// Any other category, retaining its source identity.
    Other(i32),
}
impl DeclarationCategory {
    /// Decodes the source vocabulary without guessing from labels.
    #[must_use]
    pub const fn from_source_id(id: i32) -> Self {
        match id {
            3 => Self::DonationSupport,
            other => Self::Other(other),
        }
    }
    /// Whether the category records donations and other support.
    #[must_use]
    pub const fn is_donation_support(self) -> bool {
        match self {
            Self::DonationSupport => true,
            Self::Other(_) => false,
        }
    }
    /// Encodes the original source identity.
    #[must_use]
    pub const fn source_id(self) -> i32 {
        match self {
            Self::DonationSupport => 3,
            Self::Other(id) => id,
        }
    }
}

/// Complete consistent repository input.
#[derive(Clone, Debug)]
pub struct MemberHistory {
    /// Latest member profile.
    pub member: MemberProfile,
    /// All declarations in registration order.
    pub declarations: Vec<MemberDeclaration>,
}

/// Exact amounts and coverage for one category and currency.
#[derive(Clone, Debug)]
pub struct CategoryTotal {
    /// Source identity.
    pub category: DeclarationCategory,
    /// Original category label.
    pub category_name: String,
    /// Known amounts, unavailable when no amount is known.
    pub amount: Option<BigDecimal>,
    /// Funding occurrences.
    pub entry_count: usize,
    /// Occurrences without an amount.
    pub unknown_amount_count: usize,
}

/// Independent amounts for a known currency.
#[derive(Clone, Debug)]
pub struct MemberCurrencyTotal {
    /// Normalized source currency.
    pub currency: String,
    /// Exact total, including zero and negative amounts.
    pub amount: Option<BigDecimal>,
    /// Funding occurrences.
    pub entry_count: usize,
    /// Occurrences without an amount.
    pub unknown_amount_count: usize,
    /// All represented categories.
    pub categories: Vec<CategoryTotal>,
}

/// Declared GBP amounts and coverage for one registration period.
#[derive(Clone, Debug)]
pub struct DeclaredGbpPeriodTotal {
    /// Exact known GBP sum; absence differs from zero.
    pub amount: Option<BigDecimal>,
    /// All declarations, including those without entries.
    pub declaration_count: usize,
    /// Unknown currency or a GBP entry without an amount.
    pub incomplete: bool,
}

/// Backend-owned declared GBP headlines across all categories.
#[derive(Clone, Debug)]
pub struct DeclaredGbpTotals {
    /// Inclusive registration boundary; undated records are historical only.
    pub current_term_start: DateTime<Utc>,
    /// Current parliament declarations.
    pub current_term: DeclaredGbpPeriodTotal,
    /// Complete stored history.
    pub historical: DeclaredGbpPeriodTotal,
}

/// Coverage and exact totals across complete stored history.
#[derive(Clone, Debug)]
pub struct MemberSummary {
    /// Authoritative declared GBP headlines.
    pub declared_totals: DeclaredGbpTotals,
    /// Separate currencies without conversion.
    pub currencies: Vec<MemberCurrencyTotal>,
    /// All funding occurrences.
    pub entry_count: usize,
    /// Occurrences whose currency is unknown.
    pub unknown_currency_count: usize,
    /// Earliest known registration timestamp.
    pub first_registered_at: Option<DateTime<Utc>>,
    /// Latest known registration timestamp.
    pub last_registered_at: Option<DateTime<Utc>>,
    /// Declarations without a registration timestamp.
    pub undated_declaration_count: usize,
}

/// Member summary and complete history from one consistent read.
#[derive(Clone, Debug)]
pub struct MemberDetails {
    /// Latest profile.
    pub member: MemberProfile,
    /// Complete ordered declarations.
    pub declarations: Vec<MemberDeclaration>,
    /// Exact complete-history totals.
    pub summary: MemberSummary,
}

/// Canonical funder profile with recorded names.
#[derive(Clone, Debug)]
pub struct FunderProfile {
    /// Persistent funder.
    pub funder: Funder,
    /// Company identifier with leading zeros intact.
    pub company_number: Option<String>,
    /// Every stored exact name, in stable order.
    pub aliases: Vec<String>,
    /// Source funders connected by recorded names, in stable order.
    pub member_ids: Vec<FunderId>,
    /// Direct shared-name evidence, without inferred transitive edges.
    pub alias_links: Vec<FunderAliasLink>,
    /// Reviewed organization-family connections, separate from shared names.
    pub family_links: Vec<FunderFamilyLink>,
}

/// Reviewed family membership connecting two source funders.
#[derive(Clone, Debug)]
pub struct FunderFamilyLink {
    /// First source funder.
    pub left_funder_id: FunderId,
    /// Second source funder.
    pub right_funder_id: FunderId,
    /// Persistent reviewed family key.
    pub family_key: String,
    /// Exact canonical name of the first source funder.
    pub left_name: String,
    /// Exact canonical name of the second source funder.
    pub right_name: String,
}

/// Exact recorded names responsible for a direct connection.
#[derive(Clone, Debug)]
pub struct FunderAliasLink {
    /// First source funder.
    pub left_funder_id: FunderId,
    /// Second source funder.
    pub right_funder_id: FunderId,
    /// Shared name after case and whitespace normalization.
    pub normalized_alias: String,
    /// Exact name recorded for the first funder.
    pub left_alias: String,
    /// Exact name recorded for the second funder.
    pub right_alias: String,
}

/// Compact repository aggregate for one recipient and currency.
#[derive(Clone, Debug)]
pub struct RecipientAllocation {
    /// Latest stored recipient profile.
    pub member: MemberProfile,
    /// Source currency when recorded; unknown currencies never have totals.
    pub currency: Option<String>,
    /// Sum of known amounts, or unavailable if none are known.
    pub known_total: Option<BigDecimal>,
    /// All source occurrences.
    pub entry_count: i64,
    /// Occurrences with missing amounts.
    pub unknown_amount_count: i64,
}

/// Recipient and source declaration containing only this funder's occurrences.
#[derive(Clone, Debug)]
pub struct FunderDeclaration {
    /// Latest recorded recipient profile.
    pub member: MemberProfile,
    /// Source declaration with matching occurrences, preserving duplicates.
    pub declaration: MemberDeclaration,
}

/// Repository input for a consistent funder summary and history.
#[derive(Clone, Debug)]
pub struct FunderFunding {
    /// Persistent funder identity.
    pub profile: FunderProfile,
    /// Source declarations in registration order.
    pub declarations: Vec<FunderDeclaration>,
}

/// Support grouped by recipients' latest stored party.
#[derive(Clone, Debug)]
pub struct PartyTotal {
    /// Latest stored party identity.
    pub party_id: i32,
    /// Latest stored party name.
    pub party_name: String,
    /// Total of known amounts only.
    pub amount: Option<BigDecimal>,
    /// Funding occurrences.
    pub entry_count: i64,
    /// Occurrences with unknown amounts.
    pub unknown_amount_count: i64,
}

/// Independent totals and rankings for a single currency.
#[derive(Clone, Debug)]
pub struct CurrencyBreakdown {
    /// Exact complete-history total before recipient ranking.
    pub amount: Option<BigDecimal>,
    /// Source currency.
    pub currency: String,
    /// All recorded party totals.
    pub parties: Vec<PartyTotal>,
    /// Highest known totals, followed by unavailable totals if space remains.
    pub top_recipients: Vec<RecipientAllocation>,
}

/// Funder summary without currency conversion or historical party inference.
#[derive(Clone, Debug)]
pub struct FunderDetails {
    /// Complete source history with matching funding occurrences only.
    pub declarations: Vec<FunderDeclaration>,
    /// Persistent funder identity.
    pub profile: FunderProfile,
    /// Summaries for each known currency.
    pub currencies: Vec<CurrencyBreakdown>,
    /// All funding occurrences.
    pub entry_count: i64,
    /// Occurrences excluded because currency is unknown.
    pub unknown_currency_count: i64,
    /// Maximum recipients per currency.
    pub recipient_limit: usize,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn member_requests_preserve_identifier_bounds() {
        for (value, expected) in [("1", 1), ("2147483647", i32::MAX as u32)] {
            assert_eq!(
                value.parse::<MemberDetailsRequest>().unwrap().id().value(),
                expected
            );
        }
        for value in ["", "0", "-1", "invalid", "2147483648", "4294967296"] {
            let error = value.parse::<MemberDetailsRequest>().unwrap_err();
            assert!(matches!(error, EntityDetailsError::InvalidMemberId(_)));
            assert_eq!(error.to_string(), "Invalid MP ID");
            assert!(error.source().unwrap().is::<MemberIdError>());
        }
    }

    #[test]
    fn funder_requests_preserve_uuid_identity_and_parse_source() {
        for value in [
            "00000000-0000-0000-0000-000000000000",
            "ffffffff-ffff-ffff-ffff-ffffffffffff",
        ] {
            assert_eq!(
                value.parse::<FunderDetailsRequest>().unwrap().id(),
                value.parse::<FunderId>().unwrap()
            );
        }
        for value in ["", "invalid", "00000000-0000-0000-0000-00000000000"] {
            let error = value.parse::<FunderDetailsRequest>().unwrap_err();
            assert!(matches!(error, EntityDetailsError::InvalidFunderId(_)));
            assert_eq!(error.to_string(), "Invalid funder ID");
            let source = error
                .source()
                .unwrap()
                .downcast_ref::<uuid::Error>()
                .unwrap();
            assert_eq!(source, &value.parse::<FunderId>().unwrap_err());
        }
    }
}
