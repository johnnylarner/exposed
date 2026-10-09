//! Funding occurrences and source-scoped funder observations.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use super::{
    declaration_ingestion::{CapturedDeclaration, CapturedFundingEntry, DeclarationId},
    entity_ingestion::EntityIngestionError,
    funder_name::FunderNameFeatures,
    parliament_member::MemberId,
};

/// One source member's replayed declarations, including an empty partition.
pub struct CapturedMemberDeclarations {
    pub(crate) member: MemberId,
    pub(crate) declarations: Vec<DeclarationEvidence>,
}
impl CapturedMemberDeclarations {
    /// Retains unique declarations in source identity order.
    ///
    /// # Errors
    /// Rejects repeated source declaration identities.
    pub fn new(
        member: MemberId,
        mut declarations: Vec<DeclarationEvidence>,
    ) -> Result<Self, EntityIngestionError> {
        declarations.sort_by_key(|row| row.declaration.id());
        if declarations
            .windows(2)
            .any(|rows| rows[0].declaration.id() == rows[1].declaration.id())
        {
            return Err(EntityIngestionError::DataError(
                "duplicate source declaration identity".into(),
            ));
        }
        Ok(Self {
            member,
            declarations,
        })
    }
}

/// Top-level names not represented by a top-level funding occurrence.
#[derive(Debug)]
pub struct DeclarationFunderNames {
    /// Explicit donor name.
    pub donor_name: Option<String>,
    /// Explicit payer name.
    pub payer_name: Option<String>,
    /// Explicit ultimate payer name.
    pub ultimate_payer_name: Option<String>,
    /// Source donor kind, without inference.
    pub donor_kind: Option<String>,
    /// Source donor company identifier, without inference.
    pub donor_company_number: Option<String>,
}

/// Replayed evidence with source-JSON pointers for each occurrence.
pub struct DeclarationEvidence {
    pub(crate) declaration: CapturedDeclaration,
    pub(crate) declaration_funders: Option<DeclarationFunderNames>,
    pub(crate) declaration_source_pointer: String,
    funding_source_pointers: Vec<String>,
    pub(crate) addresses: std::collections::BTreeMap<String, RoleAddresses>,
}

impl DeclarationEvidence {
    /// Associates a parsed declaration with pointers to its source fields.
    ///
    /// # Errors
    /// Rejects a different number of funding occurrences and source pointers.
    pub fn new(
        declaration: CapturedDeclaration,
        declaration_funders: Option<DeclarationFunderNames>,
        declaration_source_pointer: String,
        funding_source_pointers: Vec<String>,
    ) -> Result<Self, EntityIngestionError> {
        if declaration.funding_entries().len() != funding_source_pointers.len() {
            return Err(EntityIngestionError::DataError(
                "funding source pointers do not match occurrences".into(),
            ));
        }
        Ok(Self {
            declaration,
            declaration_funders,
            declaration_source_pointer,
            funding_source_pointers,
            addresses: std::collections::BTreeMap::new(),
        })
    }

    /// Captured projection used to check the existing raw rows.
    #[must_use]
    pub const fn declaration(&self) -> &CapturedDeclaration {
        &self.declaration
    }

    fn funding_occurrences(&self) -> impl Iterator<Item = (&CapturedFundingEntry, &str)> {
        self.declaration
            .funding_entries()
            .iter()
            .zip(self.funding_source_pointers.iter().map(String::as_str))
    }
}

/// Public address evidence owned by one checked source scope.
#[derive(Clone, Debug, Default)]
pub struct RoleAddresses {
    pub(crate) donor: Option<(String, String)>,
    pub(crate) payer: Option<(String, String)>,
    pub(crate) ultimate: Option<(String, String)>,
}

/// Whether public address evidence supports the conservative automatic-link rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressMatchQuality {
    /// No usable public-address text.
    #[default]
    Unavailable,
    /// Public text lacks a numbered street address.
    Partial,
    /// A house or building number and a street designation are both present.
    NumberedStreet,
}
impl AddressMatchQuality {
    pub(crate) fn from_normalized(address: Option<&str>) -> Self {
        let Some(address) = address else {
            return Self::Unavailable;
        };
        let tokens = address
            .split_whitespace()
            .map(|token| token.trim_matches(|c: char| !c.is_alphanumeric()))
            .collect::<Vec<_>>();
        let number = tokens.iter().any(|token| {
            token.as_bytes().first().is_some_and(u8::is_ascii_digit)
                && token.bytes().all(|c| c.is_ascii_alphanumeric())
                && token.bytes().filter(u8::is_ascii_alphabetic).count() <= 1
        });
        let street = tokens.iter().any(|token| {
            matches!(
                *token,
                "road"
                    | "rd"
                    | "street"
                    | "st"
                    | "avenue"
                    | "ave"
                    | "lane"
                    | "drive"
                    | "way"
                    | "place"
                    | "close"
                    | "terrace"
                    | "court"
                    | "crescent"
                    | "square"
                    | "gardens"
                    | "park"
                    | "mews"
                    | "boulevard"
                    | "row"
            )
        });
        if number && street {
            Self::NumberedStreet
        } else {
            Self::Partial
        }
    }
}

/// Raw public address and the usable full-address comparison value.
#[derive(Clone, Debug, Default, Serialize)]
pub struct PublicAddressEvidence {
    #[serde(rename = "address_raw")]
    pub(crate) raw: Option<String>,
    #[serde(rename = "address_normalized")]
    pub(crate) normalized: Option<String>,
    #[serde(rename = "address_source_field")]
    pub(crate) source_field: Option<String>,
    #[serde(rename = "address_match_quality")]
    pub(crate) match_quality: AddressMatchQuality,
}

impl PublicAddressEvidence {
    fn from_source(source: Option<&(String, String)>) -> Self {
        let Some((raw, field)) = source else {
            return Self::default();
        };
        let normalized = raw
            .nfkc()
            .collect::<String>()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let usable = !normalized.is_empty()
            && !matches!(
                normalized.as_str(),
                "withheld"
                    | "confidential"
                    | "private"
                    | "private address"
                    | "address private"
                    | "home address withheld"
                    | "confidential address"
                    | "not provided"
                    | "not-provided"
                    | "not disclosed"
                    | "address withheld"
                    | "not applicable"
                    | "n/a"
            );
        let normalized = usable.then_some(normalized);
        Self {
            match_quality: AddressMatchQuality::from_normalized(normalized.as_deref()),
            raw: Some(raw.clone()),
            normalized,
            source_field: Some(field.clone()),
        }
    }
}

/// Stable source occurrence identity, independent of name features.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FundingEntryId(String);

/// Stable source-role observation identity, independent of name features.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FunderObservationId(String);

#[derive(Clone, Copy)]
pub(crate) struct SourceScope {
    member: MemberId,
    declaration: DeclarationId,
    register: u32,
}
impl SourceScope {
    pub(crate) const fn new(member: MemberId, declaration: DeclarationId, register: u32) -> Self {
        Self {
            member,
            declaration,
            register,
        }
    }
    fn prefix(self) -> String {
        format!(
            "{}/{}/{}",
            self.member,
            self.declaration.value(),
            self.register
        )
    }
    fn declaration_scope(self) -> String {
        format!("{}/declaration", self.prefix())
    }
    pub(crate) fn funding_entry(self, ordinal: u32) -> FundingEntryId {
        FundingEntryId(format!("{}/funding/{ordinal}", self.prefix()))
    }
    pub(crate) fn declaration_observation(self, role: FunderRole) -> FunderObservationId {
        FunderObservationId::for_scope(&self.declaration_scope(), role)
    }
}

impl FundingEntryId {
    pub(crate) fn observation(&self, role: FunderRole) -> FunderObservationId {
        FunderObservationId::for_scope(&self.0, role)
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
impl FunderObservationId {
    fn for_scope(scope: &str, role: FunderRole) -> Self {
        Self(format!("{scope}/{}", role.suffix()))
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Role explicitly named by the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunderRole {
    /// Donor, the owner of donor-kind and company-number evidence.
    Donor,
    /// Payer, without assumed donor metadata.
    Payer,
    /// Ultimate payer, without assumed donor metadata.
    UltimatePayer,
}

impl FunderRole {
    const fn suffix(self) -> &'static str {
        match self {
            Self::Donor => "donor",
            Self::Payer => "payer",
            Self::UltimatePayer => "ultimate_payer",
        }
    }
}

/// Whether a named role belongs to a declaration or one funding occurrence.
#[derive(Debug, Serialize)]
#[serde(tag = "source_scope", rename_all = "snake_case")]
pub enum FunderObservationSource {
    /// A declaration-level scope with no top-level funding anchor.
    Declaration {
        /// JSON pointer into the retained declaration source.
        source_pointer: String,
    },
    /// One actual funding occurrence, including repeated identical entries.
    FundingEntry {
        /// Identity of the containing funding occurrence.
        funding_entry_id: FundingEntryId,
        /// Zero-based position in the replayed funding list.
        funding_ordinal: u32,
        /// JSON pointer into the retained declaration source.
        source_pointer: String,
    },
}

/// A source role and its parallel name features, without an entity-identity claim.
#[derive(Debug, Serialize)]
pub struct FunderObservation {
    pub(crate) funder_id: FunderObservationId,
    #[serde(rename = "parliament_member_id")]
    pub(crate) member_id: MemberId,
    pub(crate) declaration_id: DeclarationId,
    pub(crate) register_id: u32,
    pub(crate) role: FunderRole,
    #[serde(flatten)]
    pub(crate) source: FunderObservationSource,
    pub(crate) donor_kind: Option<String>,
    pub(crate) donor_company_number: Option<String>,
    #[serde(flatten)]
    pub(crate) name: FunderNameFeatures,
    #[serde(flatten)]
    pub(crate) address: PublicAddressEvidence,
}

/// One source funding occurrence with all original values and role references.
#[derive(Debug, Serialize)]
pub struct CleanedFundingEntry {
    pub(crate) funding_entry_id: FundingEntryId,
    #[serde(rename = "parliament_member_id")]
    pub(crate) member_id: MemberId,
    pub(crate) declaration_id: DeclarationId,
    pub(crate) register_id: u32,
    pub(crate) funding_ordinal: u32,
    pub(crate) parent_declaration_id: Option<DeclarationId>,
    pub(crate) category_id: u32,
    pub(crate) category_name: String,
    pub(crate) register_published_date: NaiveDate,
    pub(crate) registration_date: Option<NaiveDate>,
    pub(crate) fetched_at: DateTime<Utc>,
    pub(crate) donor_funder_id: Option<FunderObservationId>,
    pub(crate) payer_funder_id: Option<FunderObservationId>,
    pub(crate) ultimate_payer_funder_id: Option<FunderObservationId>,
    #[serde(flatten)]
    pub(crate) funding: CapturedFundingEntry,
}

/// Counts of the captured evidence processed by an offline clean operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarationCleaningSummary {
    /// Number of raw member partitions, including empty partitions.
    pub member_partitions: usize,
    /// Number of declarations, including those with no funding.
    pub declarations: usize,
    /// Number of actual funding occurrences.
    pub funding_entries: usize,
    /// Number of source-role observations across both scopes.
    pub funders: usize,
}

/// A coherent pair of tables constructed together by the cleaning transform.
pub struct CleanedDeclarations {
    pub(crate) funding_entries: Vec<CleanedFundingEntry>,
    pub(crate) funders: Vec<FunderObservation>,
    summary: DeclarationCleaningSummary,
}

impl CleanedDeclarations {
    /// Counts derived from the constructed tables and their input evidence.
    #[must_use]
    pub const fn summary(&self) -> &DeclarationCleaningSummary {
        &self.summary
    }

    /// Separates occurrences and observations without merging names or attributing parents.
    ///
    /// # Errors
    /// Returns an error if a source declaration has more than `u32::MAX` funding entries.
    pub fn from_captures(
        captures: &[CapturedMemberDeclarations],
    ) -> Result<Self, EntityIngestionError> {
        let mut funding_entries = Vec::new();
        let mut funders = Vec::new();
        let mut declaration_count = 0;
        for capture in captures {
            let CapturedMemberDeclarations {
                member,
                declarations,
            } = capture;
            declaration_count += declarations.len();
            for evidence in declarations {
                let declaration = &evidence.declaration;
                let scope = SourceScope::new(*member, declaration.id(), declaration.register_id());
                if let Some(names) = &evidence.declaration_funders {
                    let source = || FunderObservationSource::Declaration {
                        source_pointer: evidence.declaration_source_pointer.clone(),
                    };
                    add_roles(
                        &mut funders,
                        *member,
                        declaration,
                        &scope.declaration_scope(),
                        source,
                        names.donor_name.as_deref(),
                        names.payer_name.as_deref(),
                        names.ultimate_payer_name.as_deref(),
                        names.donor_kind.as_deref(),
                        names.donor_company_number.as_deref(),
                        evidence.addresses.get(&evidence.declaration_source_pointer),
                    );
                }
                for (ordinal, (funding, pointer)) in evidence.funding_occurrences().enumerate() {
                    let ordinal = u32::try_from(ordinal)
                        .map_err(|error| EntityIngestionError::DataError(error.to_string()))?;
                    let id = scope.funding_entry(ordinal);
                    let source = || FunderObservationSource::FundingEntry {
                        funding_entry_id: id.clone(),
                        funding_ordinal: ordinal,
                        source_pointer: pointer.to_owned(),
                    };
                    let [donor, payer, ultimate] = add_roles(
                        &mut funders,
                        *member,
                        declaration,
                        &id.0,
                        source,
                        funding.donor_name(),
                        funding.payer_name(),
                        funding.ultimate_payer_name(),
                        funding.funder_kind(),
                        funding.company_number(),
                        evidence.addresses.get(pointer),
                    );
                    funding_entries.push(CleanedFundingEntry {
                        funding_entry_id: id,
                        member_id: *member,
                        declaration_id: declaration.id(),
                        register_id: declaration.register_id(),
                        funding_ordinal: ordinal,
                        parent_declaration_id: declaration.parent_id(),
                        category_id: declaration.category_id(),
                        category_name: declaration.category_name().to_owned(),
                        register_published_date: declaration.register_published_date(),
                        registration_date: declaration.registration_date(),
                        fetched_at: declaration.fetched_at(),
                        donor_funder_id: donor,
                        payer_funder_id: payer,
                        ultimate_payer_funder_id: ultimate,
                        funding: funding.clone(),
                    });
                }
            }
        }
        let summary = DeclarationCleaningSummary {
            member_partitions: captures.len(),
            declarations: declaration_count,
            funding_entries: funding_entries.len(),
            funders: funders.len(),
        };
        Ok(Self {
            funding_entries,
            funders,
            summary,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn add_roles(
    funders: &mut Vec<FunderObservation>,
    member: MemberId,
    declaration: &CapturedDeclaration,
    scope_id: &str,
    source: impl Fn() -> FunderObservationSource,
    donor: Option<&str>,
    payer: Option<&str>,
    ultimate: Option<&str>,
    kind: Option<&str>,
    company: Option<&str>,
    addresses: Option<&RoleAddresses>,
) -> [Option<FunderObservationId>; 3] {
    [
        (FunderRole::Donor, donor, kind, company),
        (FunderRole::Payer, payer, None, None),
        (FunderRole::UltimatePayer, ultimate, None, None),
    ]
    .map(|(role, name, kind, company)| {
        if name.is_none() && kind.is_none() && company.is_none() {
            return None;
        }
        let id = FunderObservationId::for_scope(scope_id, role);
        funders.push(FunderObservation {
            funder_id: id.clone(),
            member_id: member,
            declaration_id: declaration.id(),
            register_id: declaration.register_id(),
            role,
            source: source(),
            donor_kind: kind.map(str::to_owned),
            donor_company_number: company.map(str::to_owned),
            name: FunderNameFeatures::from_name(name),
            address: PublicAddressEvidence::from_source(addresses.and_then(
                |addresses| match role {
                    FunderRole::Donor => addresses.donor.as_ref(),
                    FunderRole::Payer => addresses.payer.as_ref(),
                    FunderRole::UltimatePayer => addresses.ultimate.as_ref(),
                },
            )),
        });
        Some(id)
    })
}
