//! Deserialize and project the Interests API response.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::Value;

use super::{ApiError, ParliamentApiClient};
use crate::domain::models::declaration_ingestion::{
    CapturedDeclaration, CapturedFundingEntry, DeclarationId, StoredMember,
};

const INTERESTS_URL: &str = "https://interests-api.parliament.uk/api/v2/Interests";

#[cfg(test)]
mod tests;

impl ParliamentApiClient {
    pub(super) async fn capture_declarations(
        &self,
        member: StoredMember,
    ) -> Result<Vec<CapturedDeclaration>, ApiError> {
        let mut declarations = Vec::new();
        let mut offset = 0;
        loop {
            let url = format!(
                "{INTERESTS_URL}?MemberId={}&Type=Commons&ExcludeExpired=false&ExpandChildInterests=false&Skip={offset}&Take={}",
                member.parliament_member_id(),
                self.batch_size,
            );
            let bytes = self.client.get(url).send().await?.bytes().await?;
            let response: InterestPage = serde_json::from_slice(&bytes)?;
            if response.items.is_empty() {
                break;
            }
            let fetched_at = Utc::now();
            offset += response.items.len();
            for source in response.items {
                declarations.push(decode_declaration(source, fetched_at)?);
            }
            if offset >= response.total_results {
                break;
            }
        }
        Ok(declarations)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InterestPage {
    total_results: usize,
    // Retain every source field alongside the typed projection below.
    items: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Interest {
    id: u32,
    parent_interest_id: Option<u32>,
    category: Category,
    versions: Vec<Version>,
}

#[derive(Deserialize)]
struct Category {
    id: u32,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Version {
    register: Register,
    registration_date: Option<NaiveDate>,
    fields: Option<Vec<SourceField>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Register {
    id: u32,
    published_date: NaiveDate,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceField {
    name: String,
    #[serde(default)]
    value: Value,
    type_info: Option<Currency>,
    values: Option<Vec<Vec<Self>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Currency {
    currency_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FundingFields {
    ultimate_payer_name: Option<String>,
    donor_name: Option<String>,
    name: Option<String>,
    payer_name: Option<String>,
    value: Option<Amount>,
    payment_type: Option<String>,
    donor_status: Option<String>,
    donor_company_identifier: Option<String>,
    is_ultimate_payer_different: Option<bool>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Amount {
    Text(String),
    Number(serde_json::Number),
}

impl Amount {
    fn into_text(self) -> String {
        match self {
            Self::Text(value) => value,
            Self::Number(value) => value.to_string(),
        }
    }
}

fn decode_declaration(
    source: Value,
    fetched_at: DateTime<Utc>,
) -> Result<CapturedDeclaration, ApiError> {
    let source_json = source.to_string();
    let source: Interest = serde_json::from_value(source)?;
    let latest = source
        .versions
        .into_iter()
        .max_by_key(|version| (version.register.published_date, version.register.id))
        .ok_or_else(|| ApiError::ResponseError("declaration has no published version".into()))?;
    let funding_entries = funding_entries(&latest.fields.unwrap_or_default())?;
    Ok(CapturedDeclaration::new(
        DeclarationId::new(source.id)?,
        source
            .parent_interest_id
            .map(DeclarationId::new)
            .transpose()?,
        source.category.id,
        source.category.name,
        latest.register.id,
        latest.register.published_date,
        latest.registration_date,
        funding_entries,
        fetched_at,
        source_json,
    )?)
}

fn funding_entries(fields: &[SourceField]) -> Result<Vec<CapturedFundingEntry>, ApiError> {
    let mut entries = Vec::new();
    if fields
        .iter()
        .any(|field| matches!(field.name.as_str(), "Value" | "PaymentType"))
    {
        entries.push(parse_funding_entry(fields, false)?);
    }
    for field in fields {
        for nested in field.values.iter().flatten() {
            if field.name == "Donors" {
                entries.push(parse_funding_entry(nested, true)?);
            } else {
                entries.extend(funding_entries(nested)?);
            }
        }
    }
    Ok(entries)
}

fn parse_funding_entry(
    fields: &[SourceField],
    donor: bool,
) -> Result<CapturedFundingEntry, ApiError> {
    let values = fields
        .iter()
        .map(|field| (field.name.clone(), field.value.clone()))
        .collect();
    let funding: FundingFields = serde_json::from_value(Value::Object(values))?;
    let currency = fields
        .iter()
        .find(|field| field.name == "Value")
        .and_then(|field| field.type_info.as_ref())
        .and_then(|info| info.currency_code.clone());
    let donor_name = funding
        .donor_name
        .or(if donor { funding.name } else { None });
    Ok(CapturedFundingEntry::new(
        funding.ultimate_payer_name,
        donor_name,
        funding.payer_name,
        funding.value.map(Amount::into_text),
        currency,
        funding.payment_type,
        funding.donor_status,
        funding.donor_company_identifier,
        funding.is_ultimate_payer_different,
    ))
}
