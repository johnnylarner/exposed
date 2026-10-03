//! Interests API transport, envelope decoding, and lossless source projection.

use std::{collections::HashMap, time::Duration};

use anyhow::{Context, ensure};
use chrono::{NaiveDate, Utc};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, value::RawValue};

use super::ParliamentApiClient;
use crate::domain::{
    models::declaration_ingestion::{
        CapturedDeclaration, CapturedVersion, DeclarationId, SourceFieldGroup, StoredMember,
    },
    repositories::parliament_api::ParliamentApiError,
};

const INTERESTS_URL: &str = "https://interests-api.parliament.uk/api/v2/Interests";

#[cfg(test)]
mod tests;

impl ParliamentApiClient {
    pub(super) async fn capture_declarations(
        &self,
        member: StoredMember,
        required_id: Option<DeclarationId>,
    ) -> Result<Vec<CapturedDeclaration>, ParliamentApiError> {
        let mut declarations: Vec<CapturedDeclaration> = Vec::new();
        let mut seen = HashMap::new();
        let mut offset = 0_usize;
        loop {
            let context = format!(
                "member {}, declaration filter {:?}, page offset {offset}",
                member.parliament_member_id(),
                required_id.map(DeclarationId::value),
            );
            let mut url = format!(
                "{INTERESTS_URL}?MemberId={}&Type=Commons&ExcludeExpired=false&ExpandChildInterests=false&Skip={offset}&Take={}",
                member.parliament_member_id(),
                self.batch_size,
            );
            if let Some(id) = required_id {
                use std::fmt::Write;
                write!(url, "&InterestIds={}", id.value())
                    .expect("writing to a String cannot fail");
            }
            let bytes = self.interests_response(&url, &context).await?;
            let fetched_at = Utc::now();
            let decode = || -> anyhow::Result<(InterestPage, Vec<CapturedDeclaration>)> {
                let page: InterestPage = serde_json::from_slice(&bytes)?;
                ensure!(
                    page.skip == offset,
                    "response skip {} does not match requested offset",
                    page.skip
                );
                ensure!(
                    page.take > 0 && page.items.len() <= page.take,
                    "invalid page size"
                );
                let records = page
                    .items
                    .iter()
                    .map(|raw| {
                        decode_declaration(raw, member, fetched_at).context("decoding declaration")
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok((page, records))
            };
            let (page, records) = decode().map_err(|e| api_error(&context, &e))?;
            if records.is_empty() {
                break;
            }
            let previous_count = declarations.len();
            for declaration in records {
                if let Some(&index) = seen.get(&declaration.id()) {
                    let previous: &CapturedDeclaration = &declarations[index];
                    if previous.source_json() != declaration.source_json() {
                        return Err(ParliamentApiError::ApiError(format!(
                            "{context}: declaration {} changed during pagination; retry with a new ingestion key",
                            declaration.id().value(),
                        )));
                    }
                } else {
                    seen.insert(declaration.id(), declarations.len());
                    declarations.push(declaration);
                }
            }
            if declarations.len() == previous_count {
                return Err(ParliamentApiError::ApiError(format!(
                    "{context}: pagination stalled; no new declaration identities"
                )));
            }
            offset = offset.checked_add(page.items.len()).ok_or_else(|| {
                ParliamentApiError::ApiError(format!("{context}: pagination offset overflow"))
            })?;
            if offset >= page.total_results {
                break;
            }
        }
        Ok(declarations)
    }

    async fn interests_response(
        &self,
        url: &str,
        context: &str,
    ) -> Result<Vec<u8>, ParliamentApiError> {
        for attempt in 1..=3_u32 {
            let response = async {
                self.client
                    .get(url)
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await
            }
            .await;
            match response {
                Ok(bytes) => return Ok(bytes.to_vec()),
                Err(error) => {
                    let transient = error.is_timeout()
                        || error.is_connect()
                        || error.is_body()
                        || error.is_request()
                        || error.status().is_some_and(|status| {
                            status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
                        });
                    if !transient || attempt == 3 {
                        return Err(ParliamentApiError::ApiError(format!(
                            "{context}: request failed after {attempt} attempt(s): {error}"
                        )));
                    }
                    tokio::time::sleep(Duration::from_secs(u64::from(attempt))).await;
                }
            }
        }
        unreachable!("the last attempt returns its error")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InterestPage {
    skip: usize,
    take: usize,
    total_results: usize,
    items: Vec<Box<RawValue>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Interest {
    id: u32,
    parent_interest_id: Option<u32>,
    category: Category,
    registrant: Registrant,
    versions: Vec<Version>,
}

#[derive(Deserialize)]
struct Category {
    id: u32,
    name: String,
    r#type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Registrant {
    r#type: String,
    member_detail: MemberIdentity,
}

#[derive(Deserialize)]
struct MemberIdentity {
    id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Version {
    register: Register,
    registration_date: Option<String>,
    fields: Option<Vec<SourceField>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Register {
    id: u32,
    published_date: String,
    r#type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceField {
    name: String,
    value: Option<Box<RawValue>>,
    type_info: Option<Value>,
    values: Option<Vec<Vec<Self>>>,
}

fn decode_declaration(
    raw: &RawValue,
    member: StoredMember,
    fetched_at: chrono::DateTime<Utc>,
) -> anyhow::Result<CapturedDeclaration> {
    let source: Interest = serde_json::from_str(raw.get())?;
    ensure!(
        source.category.r#type == "Commons" && source.registrant.r#type == "Member",
        "expected a Commons member declaration, received {}",
        source.id
    );
    let versions = source
        .versions
        .into_iter()
        .enumerate()
        .map(|(index, version)| {
            ensure!(
                version.register.r#type == "Commons",
                "declaration {}: expected a Commons register",
                source.id
            );
            let path = format!("/versions/{index}/fields");
            let fields = version.fields.unwrap_or_default();
            let mut groups = vec![project_group(&fields, &path, false)?];
            donor_groups(&fields, &path, &mut groups)?;
            Ok(CapturedVersion::new(
                u32::try_from(index)?,
                version.register.id,
                NaiveDate::parse_from_str(&version.register.published_date, "%Y-%m-%d")?,
                version
                    .registration_date
                    .as_deref()
                    .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
                    .transpose()?,
                groups,
            )?)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(CapturedDeclaration::new(
        member,
        source.registrant.member_detail.id,
        DeclarationId::new(source.id)?,
        source
            .parent_interest_id
            .map(DeclarationId::new)
            .transpose()?,
        source.category.id,
        source.category.name,
        versions,
        fetched_at,
        raw.get().to_owned(),
    )?)
}

fn donor_groups(
    fields: &[SourceField],
    path: &str,
    groups: &mut Vec<SourceFieldGroup>,
) -> anyhow::Result<()> {
    for (field_index, field) in fields.iter().enumerate() {
        for (group_index, nested) in field.values.iter().flatten().enumerate() {
            let nested_path = format!("{path}/{field_index}/values/{group_index}");
            if field.name == "Donors" {
                groups.push(project_group(nested, &nested_path, true)?);
            }
            donor_groups(nested, &nested_path, groups)?;
        }
    }
    Ok(())
}

fn project_group(
    fields: &[SourceField],
    path: &str,
    donor: bool,
) -> anyhow::Result<SourceFieldGroup> {
    let field = |name: &str| -> anyhow::Result<Option<&SourceField>> {
        let mut matches = fields.iter().filter(|field| field.name == name);
        let found = matches.next();
        ensure!(
            matches.next().is_none(),
            "{path}: repeated source field {name}"
        );
        Ok(found)
    };
    let text = |name| -> anyhow::Result<Option<String>> {
        field(name)?
            .and_then(|field| field.value.as_ref())
            .map(|raw| {
                serde_json::from_str::<String>(raw.get())
                    .with_context(|| format!("{path}: {name} must contain source text"))
            })
            .transpose()
    };
    let amount_field = field("Value")?;
    let amount = amount_field
        .and_then(|field| field.value.as_ref())
        .map(|raw| match serde_json::from_str::<Value>(raw.get())? {
            Value::String(value) => Ok(value),
            Value::Number(_) => Ok(raw.get().to_owned()),
            _ => anyhow::bail!("{path}: Value must contain source decimal text or a JSON number"),
        })
        .transpose()?;
    let currency = amount_field
        .and_then(|field| field.type_info.as_ref())
        .and_then(|info| info.get("currencyCode"))
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .with_context(|| format!("{path}: currency must contain source text"))
        })
        .transpose()?;
    let different = field("IsUltimatePayerDifferent")?
        .and_then(|field| field.value.as_ref())
        .map(|raw| serde_json::from_str::<bool>(raw.get()))
        .transpose()?;
    let donor_name = match text("DonorName")? {
        Some(name) => Some(name),
        None if donor => text("Name")?,
        None => None,
    };
    Ok(SourceFieldGroup::new(
        path.to_string(),
        text("UltimatePayerName")?,
        donor_name,
        text("PayerName")?,
        amount,
        currency,
        text("PaymentType")?,
        text("DonorStatus")?,
        text("DonorCompanyIdentifier")?,
        different,
    ))
}

fn api_error(context: &str, error: &anyhow::Error) -> ParliamentApiError {
    ParliamentApiError::ApiError(format!("{context}: {error:#}"))
}
