//! Source evidence captured for later declaration interpretation.

use std::num::NonZeroU32;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::entity_ingestion::EntityIngestionError;

/// A positive source declaration identity, also used for parent references.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclarationId(NonZeroU32);

impl DeclarationId {
    /// Parses a source identity.
    ///
    /// # Errors
    /// Zero is not a declaration identity.
    pub fn new(id: u32) -> Result<Self, EntityIngestionError> {
        Ok(Self(positive_id(id)?))
    }

    /// Numeric Parliament identity.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0.get()
    }
}

/// A declaration and the funding entries from its latest publication.
#[derive(Clone, Debug)]
pub struct CapturedDeclaration {
    id: DeclarationId,
    parent_id: Option<DeclarationId>,
    category_id: NonZeroU32,
    category_name: String,
    register_id: NonZeroU32,
    register_published_date: NaiveDate,
    registration_date: Option<NaiveDate>,
    funding_entries: Vec<CapturedFundingEntry>,
    fetched_at: DateTime<Utc>,
    source_json: String,
}

impl CapturedDeclaration {
    /// Captures the latest publication and its funding entries.
    ///
    /// # Errors
    /// Rejects a missing category or register identity.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: DeclarationId,
        parent_id: Option<DeclarationId>,
        category_id: u32,
        category_name: String,
        register_id: u32,
        register_published_date: NaiveDate,
        registration_date: Option<NaiveDate>,
        funding_entries: Vec<CapturedFundingEntry>,
        fetched_at: DateTime<Utc>,
        source_json: String,
    ) -> Result<Self, EntityIngestionError> {
        Ok(Self {
            id,
            parent_id,
            category_id: positive_id(category_id)?,
            category_name,
            register_id: positive_id(register_id)?,
            register_published_date,
            registration_date,
            funding_entries,
            fetched_at,
            source_json,
        })
    }

    /// Source declaration identity, retained for future refreshes.
    #[must_use]
    pub const fn id(&self) -> DeclarationId {
        self.id
    }

    /// Parent reference as supplied by the source.
    #[must_use]
    pub const fn parent_id(&self) -> Option<DeclarationId> {
        self.parent_id
    }

    /// Source category identity.
    #[must_use]
    pub const fn category_id(&self) -> u32 {
        self.category_id.get()
    }

    /// Original category name.
    #[must_use]
    pub fn category_name(&self) -> &str {
        &self.category_name
    }

    /// Register containing the selected publication of this declaration.
    #[must_use]
    pub const fn register_id(&self) -> u32 {
        self.register_id.get()
    }

    /// Publication date of the selected register.
    #[must_use]
    pub const fn register_published_date(&self) -> NaiveDate {
        self.register_published_date
    }

    /// Registration date from the latest publication.
    #[must_use]
    pub const fn registration_date(&self) -> Option<NaiveDate> {
        self.registration_date
    }

    /// Funding entries from the latest publication, in source order.
    #[must_use]
    pub fn funding_entries(&self) -> &[CapturedFundingEntry] {
        &self.funding_entries
    }

    /// Retrieval instant in UTC.
    #[must_use]
    pub const fn fetched_at(&self) -> DateTime<Utc> {
        self.fetched_at
    }

    /// Complete source response for this declaration, including unknown fields.
    #[must_use]
    pub fn source_json(&self) -> &str {
        &self.source_json
    }
}

/// Funding details from the latest declaration publication.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapturedFundingEntry {
    ultimate_payer_name: Option<String>,
    donor_name: Option<String>,
    payer_name: Option<String>,
    amount: Option<String>,
    currency: Option<String>,
    payment_type: Option<String>,
    funder_kind: Option<String>,
    company_number: Option<String>,
    is_ultimate_payer_different: Option<bool>,
}

impl CapturedFundingEntry {
    /// Retains the supplied funding details.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        ultimate_payer_name: Option<String>,
        donor_name: Option<String>,
        payer_name: Option<String>,
        amount: Option<String>,
        currency: Option<String>,
        payment_type: Option<String>,
        funder_kind: Option<String>,
        company_number: Option<String>,
        is_ultimate_payer_different: Option<bool>,
    ) -> Self {
        Self {
            ultimate_payer_name,
            donor_name,
            payer_name,
            amount,
            currency,
            payment_type,
            funder_kind,
            company_number,
            is_ultimate_payer_different,
        }
    }
    /// Original ultimate payer name for this funding entry.
    #[must_use]
    pub fn ultimate_payer_name(&self) -> Option<&str> {
        self.ultimate_payer_name.as_deref()
    }

    /// Original donor name for this funding entry.
    #[must_use]
    pub fn donor_name(&self) -> Option<&str> {
        self.donor_name.as_deref()
    }

    /// Original payer name for this funding entry.
    #[must_use]
    pub fn payer_name(&self) -> Option<&str> {
        self.payer_name.as_deref()
    }

    /// Original amount for this funding entry.
    #[must_use]
    pub fn amount(&self) -> Option<&str> {
        self.amount.as_deref()
    }

    /// Original currency for this funding entry.
    #[must_use]
    pub fn currency(&self) -> Option<&str> {
        self.currency.as_deref()
    }

    /// Original payment type for this funding entry.
    #[must_use]
    pub fn payment_type(&self) -> Option<&str> {
        self.payment_type.as_deref()
    }

    /// Original funder kind for this funding entry.
    #[must_use]
    pub fn funder_kind(&self) -> Option<&str> {
        self.funder_kind.as_deref()
    }

    /// Original company number for this funding entry.
    #[must_use]
    pub fn company_number(&self) -> Option<&str> {
        self.company_number.as_deref()
    }

    /// Original source flag for later parent attribution.
    #[must_use]
    pub const fn is_ultimate_payer_different(&self) -> Option<bool> {
        self.is_ultimate_payer_different
    }
}

fn positive_id(id: u32) -> Result<NonZeroU32, EntityIngestionError> {
    NonZeroU32::new(id).ok_or_else(|| invalid("source identity must be positive"))
}

fn invalid(message: &str) -> EntityIngestionError {
    EntityIngestionError::DataError(message.to_string())
}

impl std::fmt::Display for DeclarationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value().fmt(f)
    }
}
