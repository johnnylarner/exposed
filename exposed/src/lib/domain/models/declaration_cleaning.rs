//! Funding occurrences and source-scoped funder observations.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::{
    declaration_ingestion::{CapturedDeclaration, CapturedFundingEntry, DeclarationId, MemberAsId},
    entity_ingestion::EntityIngestionError,
    funder_name::FunderNameFeatures,
};

/// One existing member's replayed source declarations.
pub enum CapturedMemberDeclarations {
    /// An empty member partition has no projected Parliament identity.
    Empty {
        /// Stored member UUID from the partition filename.
        member_id: Uuid,
    },
    /// A populated member partition with checked identities.
    Populated {
        /// Stored member identity from the raw partition.
        member: MemberAsId,
        /// Unique declarations in deterministic source-ID order.
        declarations: Vec<DeclarationEvidence>,
    },
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

/// Stable source occurrence identity, independent of name features.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct FundingEntryId(String);

/// Stable source-role observation identity, independent of name features.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct FunderObservationId(String);

/// Role explicitly named by the source.
#[derive(Clone, Copy, Debug, Serialize)]
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
    pub(crate) member_id: String,
    pub(crate) declaration_id: u32,
    pub(crate) register_id: u32,
    pub(crate) role: FunderRole,
    #[serde(flatten)]
    pub(crate) source: FunderObservationSource,
    pub(crate) donor_kind: Option<String>,
    pub(crate) donor_company_number: Option<String>,
    #[serde(flatten)]
    pub(crate) name: FunderNameFeatures,
}

/// One source funding occurrence with all original values and role references.
#[derive(Debug, Serialize)]
pub struct CleanedFundingEntry {
    pub(crate) funding_entry_id: FundingEntryId,
    pub(crate) member_id: String,
    pub(crate) parliament_member_id: u32,
    pub(crate) declaration_id: u32,
    pub(crate) register_id: u32,
    pub(crate) funding_ordinal: u32,
    pub(crate) parent_declaration_id: Option<u32>,
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
            let CapturedMemberDeclarations::Populated {
                member,
                declarations,
            } = capture
            else {
                continue;
            };
            declaration_count += declarations.len();
            for evidence in declarations {
                let declaration = &evidence.declaration;
                let scope_id = format!(
                    "{}/{}/{}",
                    member.member_id(),
                    declaration.id().value(),
                    declaration.register_id()
                );
                if let Some(names) = &evidence.declaration_funders {
                    let source = || FunderObservationSource::Declaration {
                        source_pointer: evidence.declaration_source_pointer.clone(),
                    };
                    add_roles(
                        &mut funders,
                        *member,
                        declaration,
                        &format!("{scope_id}/declaration"),
                        source,
                        names.donor_name.as_deref(),
                        names.payer_name.as_deref(),
                        names.ultimate_payer_name.as_deref(),
                        names.donor_kind.as_deref(),
                        names.donor_company_number.as_deref(),
                    );
                }
                for (ordinal, (funding, pointer)) in evidence.funding_occurrences().enumerate() {
                    let ordinal = u32::try_from(ordinal)
                        .map_err(|error| EntityIngestionError::DataError(error.to_string()))?;
                    let id = FundingEntryId(format!("{scope_id}/funding/{ordinal}"));
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
                    );
                    funding_entries.push(CleanedFundingEntry {
                        funding_entry_id: id,
                        member_id: member.member_id().to_string(),
                        parliament_member_id: member.parliament_member_id(),
                        declaration_id: declaration.id().value(),
                        register_id: declaration.register_id(),
                        funding_ordinal: ordinal,
                        parent_declaration_id: declaration.parent_id().map(DeclarationId::value),
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
    member: MemberAsId,
    declaration: &CapturedDeclaration,
    scope_id: &str,
    source: impl Fn() -> FunderObservationSource,
    donor: Option<&str>,
    payer: Option<&str>,
    ultimate: Option<&str>,
    kind: Option<&str>,
    company: Option<&str>,
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
        let id = FunderObservationId(format!("{scope_id}/{}", role.suffix()));
        funders.push(FunderObservation {
            funder_id: id.clone(),
            member_id: member.member_id().to_string(),
            declaration_id: declaration.id().value(),
            register_id: declaration.register_id(),
            role,
            source: source(),
            donor_kind: kind.map(str::to_owned),
            donor_company_number: company.map(str::to_owned),
            name: FunderNameFeatures::from_name(name),
        });
        Some(id)
    })
}
