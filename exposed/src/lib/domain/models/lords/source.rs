use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{
    CapturedLord, LordProfile, LordRegister, LordsCategory, LordsCategoryId, LordsError,
    RegisteredInterest, RegisteredInterestId, SourceTimestamp,
};
use crate::domain::models::parliament_member::MemberId;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    items: Vec<Item>,
    total_results: usize,
    skip: usize,
    take: usize,
}

#[derive(Deserialize)]
struct Item {
    value: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Register {
    member: Member,
    interest_categories: Option<Vec<Category>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Member {
    id: MemberId,
    name_display_as: String,
    latest_party: Option<Party>,
    latest_house_membership: Option<SourceMembership>,
}

#[derive(Deserialize)]
struct Party {
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceMembership {
    house: u32,
    membership_from: Option<String>,
    membership_status: Option<MembershipStatus>,
    membership_end_date: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MembershipStatus {
    status_description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Category {
    id: LordsCategoryId,
    name: Option<String>,
    sort_order: i32,
    interests: Option<Vec<SourceInterest>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceInterest {
    id: RegisteredInterestId,
    interest: Option<String>,
    created_when: Option<String>,
    last_amended_when: Option<String>,
    deleted_when: Option<String>,
    is_correction: bool,
    child_interests: Option<Vec<Self>>,
}

pub(super) struct ParsedPage {
    pub total: usize,
    pub skip: usize,
    pub take: usize,
    pub members: Vec<CapturedLord>,
}

pub(super) fn page(bytes: &[u8], captured_at: DateTime<Utc>) -> Result<ParsedPage, LordsError> {
    let page: Page =
        serde_json::from_slice(bytes).map_err(|error| LordsError::Source(error.to_string()))?;
    let members = page
        .items
        .into_iter()
        .map(|item| {
            Ok(CapturedLord {
                register: register(&item.value, captured_at)?,
                raw: item.value,
            })
        })
        .collect::<Result<_, LordsError>>()?;
    Ok(ParsedPage {
        total: page.total_results,
        skip: page.skip,
        take: page.take,
        members,
    })
}

pub(super) fn register(
    raw: &serde_json::Value,
    captured_at: DateTime<Utc>,
) -> Result<LordRegister, LordsError> {
    let source: Register = serde_json::from_value(raw.clone())
        .map_err(|error| LordsError::Source(error.to_string()))?;
    if source.member.name_display_as.trim().is_empty() {
        return Err(LordsError::Source("missing Lords display name".into()));
    }
    let (membership_from, membership_status, membership_end_date) =
        match source.member.latest_house_membership {
            Some(membership) => {
                if membership.house != 2 {
                    return Err(LordsError::Source(
                        "Lords source contains a non-Lords membership".into(),
                    ));
                }
                (
                    membership.membership_from,
                    membership
                        .membership_status
                        .and_then(|status| status.status_description),
                    timestamp(membership.membership_end_date)?,
                )
            }
            None => (None, None, None),
        };
    Ok(LordRegister {
        member: LordProfile {
            id: source.member.id,
            name: source.member.name_display_as,
            party_name: source.member.latest_party.and_then(|party| party.name),
            membership_from,
            membership_status,
            membership_end_date,
        },
        captured_at,
        categories: source
            .interest_categories
            .unwrap_or_default()
            .into_iter()
            .map(|category| {
                Ok(LordsCategory {
                    source_id: category.id,
                    name: category.name,
                    sort_order: category.sort_order,
                    placement: category.id.placement(),
                    interests: category
                        .interests
                        .unwrap_or_default()
                        .into_iter()
                        .map(interest)
                        .collect::<Result<_, LordsError>>()?,
                })
            })
            .collect::<Result<_, LordsError>>()?,
    })
}

fn timestamp(value: Option<String>) -> Result<Option<SourceTimestamp>, LordsError> {
    value.map(SourceTimestamp::try_from).transpose()
}

fn interest(source: SourceInterest) -> Result<RegisteredInterest, LordsError> {
    Ok(RegisteredInterest {
        source_id: source.id,
        text: source.interest,
        created_when: timestamp(source.created_when)?,
        last_amended_when: timestamp(source.last_amended_when)?,
        deleted_when: timestamp(source.deleted_when)?,
        is_correction: source.is_correction,
        children: source
            .child_interests
            .unwrap_or_default()
            .into_iter()
            .map(interest)
            .collect::<Result<_, _>>()?,
    })
}
