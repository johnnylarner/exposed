use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use super::{ImportError, Result};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Funding {
    pub funder_name: Option<String>,
    pub funder_kind: Option<String>,
    pub company_number: Option<String>,
    pub amount: Option<BigDecimal>,
    pub currency: Option<String>,
    pub payment_type: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Draft {
    pub id: i32,
    pub member_source_id: i32,
    pub category_id: i32,
    pub category_name: String,
    pub registration_date: Option<NaiveDate>,
    pub funding: Vec<Funding>,
    pub payer: Option<String>,
    pub parent_id: Option<i32>,
    pub ultimate_payer_differs: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Declaration {
    draft: Draft,
}

impl Declaration {
    pub fn draft(&self) -> &Draft {
        &self.draft
    }
}

impl Draft {
    pub fn required_parent(&self) -> Option<i32> {
        self.parent_id.filter(|_| {
            !self.ultimate_payer_differs
                && (self.funding.iter().any(|f| f.funder_name.is_none())
                    || (self.funding.is_empty() && self.payer.is_none()))
        })
    }

    pub fn accept(mut self, parent: Option<&Declaration>) -> Result<Declaration> {
        if self.id <= 0
            || self.member_source_id <= 0
            || self.category_id <= 0
            || self.category_name.trim().is_empty()
        {
            return Err(ImportError::Rejected("Invalid declaration identity".into()));
        }
        if let Some(parent_id) = self.required_parent() {
            let parent = parent
                .ok_or_else(|| ImportError::Rejected("Required parent was not returned".into()))?;
            if parent.draft.id != parent_id
                || parent.draft.member_source_id != self.member_source_id
            {
                return Err(ImportError::Rejected(
                    "Parent identity or member mismatch".into(),
                ));
            }
            let payer =
                parent.draft.payer.as_ref().ok_or_else(|| {
                    ImportError::Rejected("Required parent payer is absent".into())
                })?;
            self.payer.get_or_insert_with(|| payer.clone());
            for entry in &mut self.funding {
                entry.funder_name.get_or_insert_with(|| payer.clone());
            }
        }
        Ok(Declaration { draft: self })
    }
}

// Source alias decoding belongs to the adapter; attribution order belongs here.
pub(crate) struct FunderCandidates {
    pub ultimate_payer: Result<Option<String>>,
    pub donor: Result<Option<String>>,
    pub payer: Result<Option<String>>,
    pub group_donor: Result<Option<String>>,
}

impl FunderCandidates {
    fn preferred(self) -> Result<Option<String>> {
        for candidate in [
            self.ultimate_payer,
            self.donor,
            self.payer,
            self.group_donor,
        ] {
            if let Some(name) = candidate? {
                return Ok(Some(normalize_funder(&name)));
            }
        }
        Ok(None)
    }
}

// Decoding errors in unused fallback names/metadata must not reject an attribution.
pub(crate) fn funder_attribution(
    candidates: FunderCandidates,
    donor_status: Result<Option<String>>,
    donor_number: Result<Option<String>>,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
    let donor = match &candidates.donor {
        Ok(None) => candidates
            .group_donor
            .as_ref()
            .ok()
            .and_then(|name| name.as_ref()),
        Ok(Some(name)) => Some(name),
        Err(_) => None,
    }
    .map(|name| normalize_funder(name));
    let funder = candidates.preferred()?;
    let kind = if funder == donor { donor_status? } else { None };
    let number = if kind.as_deref() == Some("Company") {
        donor_number?
    } else {
        None
    };
    Ok((funder, kind, number))
}

pub(crate) fn normalize_funder(name: &str) -> String {
    let lowered = name.to_lowercase();
    let value = lowered.trim();
    for suffix in ["limited", "ltd.", "ltd"] {
        if let Some(prefix) = value.strip_suffix(suffix) {
            if prefix.is_empty() || prefix.ends_with(char::is_whitespace) {
                return format!("{prefix}ltd");
            }
        }
    }
    value.to_owned()
}

pub(crate) fn latest_version(versions: &[(NaiveDate, Value)]) -> Result<usize> {
    let newest =
        versions.iter().map(|v| v.0).max().ok_or_else(|| {
            ImportError::Rejected("Expected at least one published version".into())
        })?;
    let indices: Vec<_> = versions
        .iter()
        .enumerate()
        .filter(|(_, v)| v.0 == newest)
        .map(|(i, _)| i)
        .collect();
    let first = indices[0];
    if indices.iter().any(|i| versions[*i].1 != versions[first].1) {
        return Err(ImportError::Rejected(
            "Conflicting versions at latest register date".into(),
        ));
    }
    Ok(first)
}

#[derive(Clone, Debug)]
pub(crate) struct Evidence {
    pub source_id: Option<i32>,
    pub payload: Value,
    pub fetched_at: DateTime<Utc>,
    pub context: String,
}

impl Evidence {
    pub fn id(&self) -> Option<i32> {
        self.source_id
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Accepted {
    pub declaration: Declaration,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Payment {
    pub funder_id: Option<Uuid>,
    pub amount: Option<BigDecimal>,
    pub currency: Option<String>,
    pub payment_type: Option<String>,
}

pub(crate) fn same_payments(left: &[Payment], right: &[Payment]) -> bool {
    fn counts(values: &[Payment]) -> HashMap<&Payment, usize> {
        let mut counts = HashMap::new();
        for value in values {
            *counts.entry(value).or_default() += 1;
        }
        counts
    }
    counts(left) == counts(right)
}

// An omission retains metadata. An explicit non-company correction clears its number.
pub(crate) fn merge_funder_metadata(
    incoming: &Funding,
    previous_kind: Option<String>,
    previous_number: Option<String>,
) -> (Option<String>, Option<String>) {
    let kind = incoming.funder_kind.clone().or(previous_kind);
    let number = if kind.as_deref() == Some("Company") {
        incoming.company_number.clone().or(previous_number)
    } else {
        None
    };
    (kind, number)
}
