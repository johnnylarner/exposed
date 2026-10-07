//! Source decoding shared by acquisition and offline replay.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::Value;

use super::parliament_api::ApiError;
use crate::domain::models::{
    declaration_cleaning::{DeclarationEvidence, DeclarationFunderNames, RoleAddresses},
    declaration_ingestion::{CapturedDeclaration, CapturedFundingEntry, DeclarationId},
};

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

struct DecodedDeclaration {
    declaration: CapturedDeclaration,
    fields: Vec<SourceField>,
    source_pointer: String,
    funding_source_pointers: Vec<String>,
}

pub(super) fn decode_declaration(
    source: Value,
    fetched_at: DateTime<Utc>,
) -> Result<CapturedDeclaration, ApiError> {
    Ok(decode_source(source, fetched_at)?.declaration)
}

pub(super) fn replay_declaration(
    source: Value,
    fetched_at: DateTime<Utc>,
) -> Result<DeclarationEvidence, ApiError> {
    let decoded = decode_source(source, fetched_at)?;
    let declaration_funders = if has_funding_anchor(&decoded.fields) {
        None
    } else {
        let names = parse_funding_entry(&decoded.fields, false)?;
        Some(DeclarationFunderNames {
            donor_name: names.donor_name().map(str::to_owned),
            payer_name: names.payer_name().map(str::to_owned),
            ultimate_payer_name: names.ultimate_payer_name().map(str::to_owned),
            donor_kind: names.funder_kind().map(str::to_owned),
            donor_company_number: names.company_number().map(str::to_owned),
        })
    };
    let mut evidence = DeclarationEvidence::new(
        decoded.declaration,
        declaration_funders,
        decoded.source_pointer,
        decoded.funding_source_pointers,
    )?;
    collect_addresses(
        &decoded.fields,
        &evidence.declaration_source_pointer,
        false,
        &mut evidence.addresses,
    )?;
    Ok(evidence)
}

fn collect_addresses(
    fields: &[SourceField],
    pointer: &str,
    nested_donor: bool,
    addresses: &mut std::collections::BTreeMap<String, RoleAddresses>,
) -> Result<(), ApiError> {
    let mut roles = RoleAddresses::default();
    for field in fields {
        let role = match field.name.as_str() {
            "DonorPublicAddress" if !nested_donor => Some(&mut roles.donor),
            "PublicAddress" if nested_donor => Some(&mut roles.donor),
            "PayerPublicAddress" => Some(&mut roles.payer),
            "UltimatePayerAddress" => Some(&mut roles.ultimate),
            _ => None,
        };
        if let Some(role) = role
            && !field.value.is_null()
        {
            let raw = field.value.as_str().ok_or_else(|| {
                ApiError::ResponseError(format!(
                    "{} must contain a public-address string",
                    field.name
                ))
            })?;
            if role.is_some() {
                return Err(ApiError::ResponseError(format!("duplicate {}", field.name)));
            }
            *role = Some((raw.to_owned(), field.name.clone()));
        }
    }
    addresses.insert(pointer.to_owned(), roles);
    for (field_index, field) in fields.iter().enumerate() {
        for (entry_index, nested) in field.values.iter().flatten().enumerate() {
            collect_addresses(
                nested,
                &format!("{pointer}/{field_index}/values/{entry_index}"),
                field.name == "Donors",
                addresses,
            )?;
        }
    }
    Ok(())
}

fn decode_source(source: Value, fetched_at: DateTime<Utc>) -> Result<DecodedDeclaration, ApiError> {
    let source_json = source.to_string();
    let source: Interest = serde_json::from_value(source)?;
    let (version_index, latest) = source
        .versions
        .into_iter()
        .enumerate()
        .max_by_key(|(_, version)| (version.register.published_date, version.register.id))
        .ok_or_else(|| ApiError::ResponseError("declaration has no published version".into()))?;
    let pointer = format!("/versions/{version_index}/fields");
    let fields = latest.fields.unwrap_or_default();
    let mut funding = Vec::new();
    let mut pointers = Vec::new();
    funding_entries(&fields, &pointer, &mut funding, &mut pointers)?;
    let declaration = CapturedDeclaration::new(
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
        funding,
        fetched_at,
        source_json,
    )?;
    Ok(DecodedDeclaration {
        declaration,
        fields,
        source_pointer: pointer,
        funding_source_pointers: pointers,
    })
}

fn has_funding_anchor(fields: &[SourceField]) -> bool {
    fields
        .iter()
        .any(|field| matches!(field.name.as_str(), "Value" | "PaymentType"))
}

fn funding_entries(
    fields: &[SourceField],
    pointer: &str,
    entries: &mut Vec<CapturedFundingEntry>,
    pointers: &mut Vec<String>,
) -> Result<(), ApiError> {
    if has_funding_anchor(fields) {
        entries.push(parse_funding_entry(fields, false)?);
        pointers.push(pointer.to_owned());
    }
    for (field_index, field) in fields.iter().enumerate() {
        for (entry_index, nested) in field.values.iter().flatten().enumerate() {
            let pointer = format!("{pointer}/{field_index}/values/{entry_index}");
            if field.name == "Donors" {
                entries.push(parse_funding_entry(nested, true)?);
                pointers.push(pointer);
            } else {
                funding_entries(nested, &pointer, entries, pointers)?;
            }
        }
    }
    Ok(())
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

#[cfg(test)]
#[path = "parliament_api/declarations/tests.rs"]
mod tests;
