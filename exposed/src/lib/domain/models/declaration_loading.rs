//! A checked current-database projection of one resolved declaration run.

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::entity_ingestion::{EntityIngestionError, IngestionKey};

/// All rows needed to publish one resolved run in the application database.
pub struct DeclarationLoad {
    pub(crate) ingestion_key: IngestionKey,
    pub(crate) fingerprint: String,
    pub(crate) declarations: Vec<LoadDeclaration>,
    pub(crate) funders: Vec<LoadFunder>,
    pub(crate) funding_entries: Vec<LoadFundingEntry>,
}

/// One source declaration and its existing member identity.
pub struct LoadDeclaration {
    pub(crate) source_declaration_id: u32,
    pub(crate) member_id: Uuid,
    pub(crate) parliament_member_id: u32,
    pub(crate) category_id: u32,
    pub(crate) category_name: String,
    pub(crate) register_id: u32,
    pub(crate) register_published_date: NaiveDate,
    pub(crate) parent_declaration_id: Option<u32>,
    pub(crate) fetched_at: DateTime<Utc>,
    pub(crate) registration_date: Option<NaiveDate>,
}

/// One resolved identity referenced by an attributed funding occurrence.
pub struct LoadFunder {
    pub(crate) identity_id: String,
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) kind: Option<String>,
    pub(crate) company_number: Option<String>,
}

/// One source funding occurrence and its independent attribution decision.
pub struct LoadFundingEntry {
    pub(crate) source_id: String,
    pub(crate) source_declaration_id: u32,
    pub(crate) identity_id: Option<String>,
    pub(crate) selected_observation_id: Option<String>,
    pub(crate) attribution_basis: Option<String>,
    pub(crate) selected_parent_declaration_id: Option<u32>,
    pub(crate) unavailable_reason: Option<String>,
    pub(crate) issues: Vec<String>,
    pub(crate) amount: Option<BigDecimal>,
    pub(crate) currency: Option<String>,
    pub(crate) payment_type: Option<String>,
}

/// Counts committed by a declaration load.
#[derive(Clone, Debug)]
pub struct DeclarationLoadSummary {
    /// Whether this call imported the run or confirmed an exact retry.
    pub outcome: DeclarationLoadOutcome,
    /// Declarations in the run.
    pub declarations: usize,
    /// Distinct resolved identities referenced by selected attributions.
    pub funders: usize,
    /// Original funding occurrences, including unavailable attributions.
    pub funding_entries: usize,
}

/// Result of the atomic load operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarationLoadOutcome {
    /// This call committed the run.
    Imported,
    /// This run was already committed from identical artifacts.
    AlreadyLoaded,
}

impl DeclarationLoad {
    /// Constructs a run after its source records and cross-file references are checked.
    ///
    /// # Errors
    /// Rejects empty keys, duplicate IDs, invalid positive IDs, missing references, and
    /// incomplete declaration coverage.
    pub fn checked(
        ingestion_key: IngestionKey,
        fingerprint: String,
        declarations: Vec<LoadDeclaration>,
        funders: Vec<LoadFunder>,
        funding_entries: Vec<LoadFundingEntry>,
    ) -> Result<Self, EntityIngestionError> {
        if ingestion_key.uuid().is_nil() || fingerprint.is_empty() {
            return Err(invalid("declaration load fingerprint is empty"));
        }
        let declaration_ids = declarations
            .iter()
            .map(|row| row.source_declaration_id)
            .collect::<std::collections::BTreeSet<_>>();
        if declaration_ids.len() != declarations.len()
            || declarations.iter().any(|row| {
                row.source_declaration_id == 0
                    || row.member_id.is_nil()
                    || row.parliament_member_id == 0
                    || row.category_id == 0
                    || row.category_name.trim().is_empty()
                    || row.register_id == 0
                    || row.parent_declaration_id == Some(0)
            })
        {
            return Err(invalid("invalid or duplicate declaration load row"));
        }
        let funder_ids = funders
            .iter()
            .map(|row| row.identity_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if funder_ids.len() != funders.len()
            || funders.iter().any(|row| {
                row.identity_id.is_empty()
                    || (row.name.trim().is_empty()
                        && row.company_number.as_deref().is_none_or(str::is_empty))
                    || row.aliases.iter().any(|alias| alias.trim().is_empty())
            })
        {
            return Err(invalid("invalid or duplicate resolved identity"));
        }
        let mut occurrence_ids = std::collections::BTreeSet::new();
        for row in &funding_entries {
            let selected = row.selected_observation_id.is_some();
            if row.source_id.is_empty()
                || row
                    .selected_observation_id
                    .as_deref()
                    .is_some_and(str::is_empty)
                || !occurrence_ids.insert(row.source_id.as_str())
                || !declaration_ids.contains(&row.source_declaration_id)
                || row
                    .identity_id
                    .as_deref()
                    .is_some_and(|id| !funder_ids.contains(id))
                || (row.identity_id.is_some() != row.selected_observation_id.is_some())
                || row.attribution_basis.is_some() != selected
                || row.unavailable_reason.is_some() != !selected
                || row.selected_parent_declaration_id.is_some()
                    != (row.attribution_basis.as_deref() == Some("parent_payer"))
                || row.attribution_basis.as_deref().is_some_and(|basis| {
                    !matches!(
                        basis,
                        "explicit_ultimate_payer" | "parent_payer" | "donor" | "payer"
                    )
                })
            {
                return Err(invalid(
                    "incoherent funding attribution in declaration load",
                ));
            }
        }
        Ok(Self {
            ingestion_key,
            fingerprint,
            declarations,
            funders,
            funding_entries,
        })
    }

    /// Row counts for the committed run.
    #[must_use]
    pub fn summary(&self) -> DeclarationLoadSummary {
        DeclarationLoadSummary {
            outcome: DeclarationLoadOutcome::Imported,
            declarations: self.declarations.len(),
            funders: self.funders.len(),
            funding_entries: self.funding_entries.len(),
        }
    }
}

fn invalid(message: &str) -> EntityIngestionError {
    EntityIngestionError::DataError(message.to_owned())
}
