//! Source evidence captured for later declaration interpretation.

use std::num::NonZeroU32;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::entity_ingestion::{EntityIngestionError, IngestionKey};

/// An existing database member and their Parliament identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredMember {
    member_id: Uuid,
    parliament_member_id: NonZeroU32,
}

impl StoredMember {
    /// Associates a non-nil database UUID with a positive Parliament ID.
    ///
    /// # Errors
    /// Returns an error if either identity is missing.
    pub fn new(member_id: Uuid, parliament_member_id: u32) -> Result<Self, EntityIngestionError> {
        if member_id.is_nil() {
            return Err(invalid("stored member UUID must not be nil"));
        }
        Ok(Self {
            member_id,
            parliament_member_id: positive_id(parliament_member_id)?,
        })
    }

    /// Existing database identity.
    #[must_use]
    pub const fn member_id(&self) -> Uuid {
        self.member_id
    }

    /// Parliament identity used for acquisition.
    #[must_use]
    pub const fn parliament_member_id(&self) -> u32 {
        self.parliament_member_id.get()
    }
}

/// A positive source declaration identity, also used for parent references.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

/// Complete source evidence for one declaration belonging to a stored member.
#[derive(Clone, Debug)]
pub struct CapturedDeclaration {
    member: StoredMember,
    id: DeclarationId,
    parent_id: Option<DeclarationId>,
    category_id: NonZeroU32,
    category_name: String,
    versions: Vec<CapturedVersion>,
    fetched_at: DateTime<Utc>,
    source_json: String,
}

impl CapturedDeclaration {
    /// Checks source identities and association without interpreting funding values.
    ///
    /// # Errors
    /// Rejects missing identities, a different member, self-parenting, or no versions.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        member: StoredMember,
        source_member_id: u32,
        id: DeclarationId,
        parent_id: Option<DeclarationId>,
        category_id: u32,
        category_name: String,
        versions: Vec<CapturedVersion>,
        fetched_at: DateTime<Utc>,
        source_json: String,
    ) -> Result<Self, EntityIngestionError> {
        if member.parliament_member_id() != source_member_id {
            return Err(invalid(
                "declaration belongs to a different Parliament member",
            ));
        }
        if parent_id == Some(id) {
            return Err(invalid("declaration cannot be its own parent"));
        }
        if versions.is_empty() {
            return Err(invalid(
                "declaration must contain at least one source version",
            ));
        }
        Ok(Self {
            member,
            id,
            parent_id,
            category_id: positive_id(category_id)?,
            category_name,
            versions,
            fetched_at,
            source_json,
        })
    }
    /// Stored member association.
    #[must_use]
    pub const fn member(&self) -> StoredMember {
        self.member
    }

    /// Source declaration identity.
    #[must_use]
    pub const fn id(&self) -> DeclarationId {
        self.id
    }

    /// Required source parent, when present.
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

    /// All versions in source order.
    #[must_use]
    pub fn versions(&self) -> &[CapturedVersion] {
        &self.versions
    }

    /// Retrieval instant in UTC.
    #[must_use]
    pub const fn fetched_at(&self) -> DateTime<Utc> {
        self.fetched_at
    }

    /// Complete original JSON, including unknown fields.
    #[must_use]
    pub fn source_json(&self) -> &str {
        &self.source_json
    }

    /// Number of top-level and donor rows across every source version.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.versions
            .iter()
            .map(|version| version.groups.len())
            .sum()
    }
}

/// One source version with its calendar dates and ordered field groups.
#[derive(Clone, Debug)]
pub struct CapturedVersion {
    index: u32,
    register_id: NonZeroU32,
    published_date: NaiveDate,
    registration_date: Option<NaiveDate>,
    groups: Vec<SourceFieldGroup>,
}

impl CapturedVersion {
    /// Constructs a version without selecting or normalizing it.
    ///
    /// # Errors
    /// Rejects zero register identities or a missing top-level group.
    pub fn new(
        index: u32,
        register_id: u32,
        published_date: NaiveDate,
        registration_date: Option<NaiveDate>,
        groups: Vec<SourceFieldGroup>,
    ) -> Result<Self, EntityIngestionError> {
        if groups.is_empty() {
            return Err(invalid("version must retain its top-level field group"));
        }
        Ok(Self {
            index,
            register_id: positive_id(register_id)?,
            published_date,
            registration_date,
            groups,
        })
    }
    /// Captured index from the source version.
    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }

    /// Captured register id from the source version.
    #[must_use]
    pub const fn register_id(&self) -> u32 {
        self.register_id.get()
    }

    /// Captured published date from the source version.
    #[must_use]
    pub const fn published_date(&self) -> NaiveDate {
        self.published_date
    }

    /// Captured registration date from the source version.
    #[must_use]
    pub const fn registration_date(&self) -> Option<NaiveDate> {
        self.registration_date
    }

    /// Captured groups from the source version.
    #[must_use]
    pub fn groups(&self) -> &[SourceFieldGroup] {
        &self.groups
    }
}

/// A top-level or donor field group; optional strings retain their source spelling.
#[derive(Clone, Debug)]
pub struct SourceFieldGroup {
    path: String,
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

impl SourceFieldGroup {
    /// Retains fields from one source position, with no inheritance or interpretation.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        path: String,
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
            path,
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
    /// JSON pointer to the exact source field group.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Original ultimate payer name in this group.
    #[must_use]
    pub fn ultimate_payer_name(&self) -> Option<&str> {
        self.ultimate_payer_name.as_deref()
    }

    /// Original donor name in this group.
    #[must_use]
    pub fn donor_name(&self) -> Option<&str> {
        self.donor_name.as_deref()
    }

    /// Original payer name in this group.
    #[must_use]
    pub fn payer_name(&self) -> Option<&str> {
        self.payer_name.as_deref()
    }

    /// Original amount in this group.
    #[must_use]
    pub fn amount(&self) -> Option<&str> {
        self.amount.as_deref()
    }

    /// Original currency in this group.
    #[must_use]
    pub fn currency(&self) -> Option<&str> {
        self.currency.as_deref()
    }

    /// Original payment type in this group.
    #[must_use]
    pub fn payment_type(&self) -> Option<&str> {
        self.payment_type.as_deref()
    }

    /// Original funder kind in this group.
    #[must_use]
    pub fn funder_kind(&self) -> Option<&str> {
        self.funder_kind.as_deref()
    }

    /// Original company number in this group.
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

/// Counts for a member whose declaration file was successfully finalized.
#[derive(Clone, Debug)]
pub struct DeclarationMemberOutput {
    member: StoredMember,
    declaration_count: usize,
    row_count: usize,
}

impl DeclarationMemberOutput {
    /// Records a completed member, including an empty result.
    #[must_use]
    pub const fn new(member: StoredMember, declaration_count: usize, row_count: usize) -> Self {
        Self {
            member,
            declaration_count,
            row_count,
        }
    }

    /// Member captured by this output.
    #[must_use]
    pub const fn member(&self) -> StoredMember {
        self.member
    }

    /// Distinct declarations, including captured parents.
    #[must_use]
    pub const fn declaration_count(&self) -> usize {
        self.declaration_count
    }

    /// Flattened version and field-group rows.
    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.row_count
    }
}

/// Successfully completed declaration capture.
#[derive(Clone, Debug)]
pub struct DeclarationCaptureOutcome {
    ingestion_key: IngestionKey,
    output_location: String,
    members: Vec<DeclarationMemberOutput>,
}

impl DeclarationCaptureOutcome {
    /// Records the outcome after storage publishes the completion manifest.
    #[must_use]
    pub const fn new(
        ingestion_key: IngestionKey,
        output_location: String,
        members: Vec<DeclarationMemberOutput>,
    ) -> Self {
        Self {
            ingestion_key,
            output_location,
            members,
        }
    }

    /// Key identifying this completed run.
    #[must_use]
    pub const fn ingestion_key(&self) -> &IngestionKey {
        &self.ingestion_key
    }

    /// Location returned by storage.
    #[must_use]
    pub fn output_location(&self) -> &str {
        &self.output_location
    }

    /// Selected members, including members without declarations.
    #[must_use]
    pub const fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Total distinct declarations across member files.
    #[must_use]
    pub fn declaration_count(&self) -> usize {
        self.members
            .iter()
            .map(DeclarationMemberOutput::declaration_count)
            .sum()
    }

    /// Total projected rows across member files.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.members
            .iter()
            .map(DeclarationMemberOutput::row_count)
            .sum()
    }
}

fn positive_id(id: u32) -> Result<NonZeroU32, EntityIngestionError> {
    NonZeroU32::new(id).ok_or_else(|| invalid("source identity must be positive"))
}

fn invalid(message: &str) -> EntityIngestionError {
    EntityIngestionError::DataError(message.to_string())
}
