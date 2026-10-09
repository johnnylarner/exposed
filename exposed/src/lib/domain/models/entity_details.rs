//! Stored profiles, declaration occurrences, and exact currency summaries.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};

use super::{
    funder::{Funder, FunderId},
    parliament_member::MemberId,
};

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
    /// Source currency, including unknown values.
    pub currency: Option<String>,
    /// Source payment classification.
    pub payment_type: Option<String>,
}

/// A declaration with all of its funding occurrences.
#[derive(Clone, Debug)]
pub struct RecentDeclaration {
    /// Parliament declaration identity.
    pub source_id: i32,
    /// Source category.
    pub category_name: String,
    /// Registration date when available.
    pub registered_at: Option<DateTime<Utc>>,
    /// Funding occurrences, preserving duplicates.
    pub entries: Vec<DeclaredFunding>,
}

/// A member and the bounded latest declaration window.
#[derive(Clone, Debug)]
pub struct MemberDetails {
    /// Latest member profile.
    pub member: MemberProfile,
    /// Latest declarations in registration order.
    pub declarations: Vec<RecentDeclaration>,
    /// Total stored declarations.
    pub declaration_count: i64,
    /// Maximum declarations displayed.
    pub declaration_limit: usize,
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

/// Repository input for a consistent funder summary.
#[derive(Clone, Debug)]
pub struct FunderFunding {
    /// Persistent funder identity.
    pub profile: FunderProfile,
    /// One row per recipient and currency.
    pub allocations: Vec<RecipientAllocation>,
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
