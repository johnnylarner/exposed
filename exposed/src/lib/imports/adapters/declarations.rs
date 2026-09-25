use std::{collections::HashMap, str::FromStr};

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::imports::core::{
    ImportError, Result,
    declarations::{Draft, FunderCandidates, Funding, funder_attribution, latest_version},
};

pub(crate) fn invalid(path: &str, value: &Value, message: &str) -> ImportError {
    ImportError::Rejected(format!("{path}: {message}; input={value}"))
}

pub(crate) fn source_date(value: &Value, path: &str) -> Result<NaiveDate> {
    let text = value
        .as_str()
        .ok_or_else(|| invalid(path, value, "expected an ISO date"))?;
    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Ok(date);
    }
    if let Ok(date) = DateTime::parse_from_rfc3339(text) {
        return Ok(date.date_naive());
    }
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(date) = NaiveDateTime::parse_from_str(text, format) {
            return Ok(date.date());
        }
    }
    Err(invalid(path, value, "expected an ISO date or datetime"))
}

fn positive(value: &Value, path: &str) -> Result<i32> {
    value
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| invalid(path, value, "expected a positive integer"))
}

fn nonblank(value: &Value, path: &str) -> Result<String> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid(path, value, "expected a nonblank string"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Field {
    name: String,
    #[serde(rename = "type")]
    kind: Option<String>,
    type_info: Option<Map<String, Value>>,
    #[serde(default)]
    value: Value,
    values: Option<Vec<Vec<Field>>>,
}

impl Field {
    fn text(&self, path: &str) -> Result<Option<String>> {
        if self.value.is_null() {
            return Ok(None);
        }
        self.value
            .as_str()
            .map(|s| (!s.trim().is_empty()).then(|| s.to_owned()))
            .ok_or_else(|| invalid(path, &self.value, "expected a string"))
    }

    fn check_shape(&self, path: &str, allowed: bool, nested: bool) -> Result<()> {
        let currency = self
            .type_info
            .as_ref()
            .is_some_and(|m| m.contains_key("currencyCode"));
        let financial =
            ["Value", "PaymentType", "Donors"].contains(&self.name.as_str()) || currency;
        if financial
            && (!allowed || (currency && self.name != "Value") || (nested && self.name == "Donors"))
        {
            return Err(invalid(
                path,
                &self.value,
                "unsupported funding field or nesting",
            ));
        }
        for (i, group) in self.values.iter().flatten().enumerate() {
            if self.name == "Donors" && group.is_empty() {
                return Err(invalid(path, &self.value, "empty donor group"));
            }
            for (j, field) in group.iter().enumerate() {
                field.check_shape(
                    &format!("{path}.values.{i}.{j}.{}", field.name),
                    allowed && self.name == "Donors",
                    true,
                )?;
            }
        }
        Ok(())
    }
}

fn funding(fields: &[Field], path: &str, donor_group: bool) -> Result<Funding> {
    let index: HashMap<_, _> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.as_str(), (i, f)))
        .collect();
    if index.len() != fields.len() {
        return Err(ImportError::Rejected(format!(
            "{path}: repeated field names"
        )));
    }
    let text = |name: &str| -> Result<Option<String>> {
        match index.get(name) {
            Some((i, f)) => f.text(&format!("{path}.{i}.{name}.value")),
            None => Ok(None),
        }
    };
    let mut amount = None;
    let mut currency = None;
    if let Some((i, value)) = index.get("Value") {
        let location = format!("{path}.{i}.Value");
        if value.kind.as_deref() != Some("Decimal") {
            return Err(invalid(
                &location,
                &value.value,
                "expected Decimal monetary field",
            ));
        }
        if !value.value.is_null() {
            let raw = match &value.value {
                Value::String(text) => text.clone(),
                Value::Number(number) => {
                    let raw = number.to_string();
                    // Python accepts arbitrary-size JSON integers, but rejects JSON floats.
                    if raw.contains(['.', 'e', 'E']) {
                        return Err(invalid(&location, &value.value, "unsupported amount"));
                    }
                    raw
                }
                _ => return Err(invalid(&location, &value.value, "unsupported amount")),
            };
            let unsigned = raw.strip_prefix('-').unwrap_or(&raw);
            let parts: Vec<_> = unsigned.split('.').collect();
            if parts.len() > 2
                || parts
                    .iter()
                    .any(|p| p.is_empty() || !p.bytes().all(|c| c.is_ascii_digit()))
            {
                return Err(invalid(&location, &value.value, "unsupported amount"));
            }
            amount = Some(
                BigDecimal::from_str(&raw)
                    .map_err(|_| invalid(&location, &value.value, "unsupported amount"))?,
            );
        }
        if let Some(code) = value
            .type_info
            .as_ref()
            .and_then(|m| m.get("currencyCode"))
            .filter(|v| !v.is_null())
        {
            currency = Some(
                code.as_str()
                    .ok_or_else(|| invalid(&location, code, "expected a currency string"))?
                    .to_owned(),
            );
        }
    }
    let (funder_name, funder_kind, company_number) = funder_attribution(
        FunderCandidates {
            ultimate_payer: text("UltimatePayerName"),
            donor: text("DonorName"),
            payer: text("PayerName"),
            group_donor: if donor_group { text("Name") } else { Ok(None) },
        },
        text("DonorStatus"),
        text("DonorCompanyIdentifier"),
    )?;
    Ok(Funding {
        funder_name,
        funder_kind,
        company_number,
        amount,
        currency,
        payment_type: text("PaymentType")?,
    })
}

pub(crate) fn interpret(raw: &Value) -> Result<Draft> {
    let id = positive(&raw["id"], "id")?;
    let member_source_id = positive(
        &raw["registrant"]["memberDetail"]["id"],
        "registrant.memberDetail.id",
    )?;
    if raw["registrant"]["type"] != "Member" || raw["category"]["type"] != "Commons" {
        return Err(invalid(
            "registrant/category",
            raw,
            "expected a Commons Member declaration",
        ));
    }
    let category_id = positive(&raw["category"]["id"], "category.id")?;
    let category_name = nonblank(&raw["category"]["name"], "category.name")?;
    let parent_id = if raw["parentInterestId"].is_null() {
        None
    } else {
        Some(positive(&raw["parentInterestId"], "parentInterestId")?)
    };
    let versions = raw["versions"]
        .as_array()
        .ok_or_else(|| invalid("versions", &raw["versions"], "expected versions"))?;
    let published = versions
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let date = source_date(
                &v["register"]["publishedDate"],
                &format!("versions.{i}.register.publishedDate"),
            )?;
            let mut content = v
                .as_object()
                .ok_or_else(|| invalid("versions", v, "expected version object"))?
                .clone();
            content.remove("register");
            content.remove("links");
            Ok((date, Value::Object(content)))
        })
        .collect::<Result<Vec<_>>>()?;
    let selected = latest_version(&published)?;
    let version = &versions[selected];
    let path = format!("versions.{selected}.fields");
    let fields: Vec<Field> = serde_json::from_value(version["fields"].clone())
        .map_err(|e| invalid(&path, &version["fields"], &e.to_string()))?;
    let mut entries = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        field.check_shape(&format!("{path}.{i}.{}", field.name), true, false)?;
    }
    if fields
        .iter()
        .any(|f| ["Value", "PaymentType"].contains(&f.name.as_str()))
    {
        entries.push(funding(&fields, &path, false)?);
    }
    for (i, field) in fields
        .iter()
        .enumerate()
        .filter(|(_, f)| f.name == "Donors")
    {
        let groups = field
            .values
            .as_ref()
            .ok_or_else(|| invalid(&path, &field.value, "expected donor groups"))?;
        for (j, group) in groups.iter().enumerate() {
            entries.push(funding(
                group,
                &format!("{path}.{i}.Donors.values.{j}"),
                true,
            )?);
        }
    }
    let payer = funding(&fields, &path, false)?.funder_name;
    let mut ultimate_payer_differs = false;
    if parent_id.is_some() {
        if let Some(field) = fields.iter().find(|f| f.name == "IsUltimatePayerDifferent") {
            if !field.value.is_null() && !field.value.is_boolean() {
                return Err(invalid(&path, &field.value, "expected a boolean"));
            }
            ultimate_payer_differs = field.value == true;
        }
    }
    let registration_date = if version["registrationDate"].is_null() {
        None
    } else {
        Some(source_date(
            &version["registrationDate"],
            "registrationDate",
        )?)
    };
    Ok(Draft {
        id,
        member_source_id,
        category_id,
        category_name,
        registration_date,
        funding: entries,
        payer,
        parent_id,
        ultimate_payer_differs,
    })
}
