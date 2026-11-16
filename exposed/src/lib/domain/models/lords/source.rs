use serde::Deserialize;

use super::{
    LordProfile, LordRegister, LordsCategory, LordsCategoryId, LordsError, SimpleDeclaration,
    SimpleDeclarationId, SourceTimestamp,
};
use crate::domain::models::{
    membership::{Chamber, SourceMembership},
    parliament_member::MemberId,
};

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
    latest_party: Party,
    latest_house_membership: SourceMembership,
}

#[derive(Deserialize)]
struct Party {
    id: u32,
    name: String,
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
    id: SimpleDeclarationId,
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
    pub members: Vec<LordRegister>,
}

pub(super) fn page(bytes: &[u8]) -> Result<ParsedPage, LordsError> {
    let page: Page =
        serde_json::from_slice(bytes).map_err(|error| LordsError::Source(error.to_string()))?;
    let members = page
        .items
        .into_iter()
        .map(|item| register(&item.value))
        .collect::<Result<_, LordsError>>()?;
    Ok(ParsedPage {
        total: page.total_results,
        skip: page.skip,
        take: page.take,
        members,
    })
}

pub(super) fn register(raw: &serde_json::Value) -> Result<LordRegister, LordsError> {
    let source: Register = serde_json::from_value(raw.clone())
        .map_err(|error| LordsError::Source(error.to_string()))?;
    if source.member.name_display_as.trim().is_empty() {
        return Err(LordsError::Source("missing Lords display name".into()));
    }
    let membership = source
        .member
        .latest_house_membership
        .into_membership()
        .map_err(LordsError::Source)?;
    if membership.kind.chamber() != Chamber::Lords {
        return Err(LordsError::Source(
            "Lords source contains a non-Lords membership".into(),
        ));
    }
    Ok(LordRegister {
        member: LordProfile {
            id: source.member.id,
            name: source.member.name_display_as,
            parliamentary_name: None,
            party_id: source.member.latest_party.id,
            party_name: source.member.latest_party.name,
            membership,
        },
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
    value
        .map(SourceTimestamp::try_from)
        .transpose()
        .map_err(Into::into)
}

fn interest(source: SourceInterest) -> Result<SimpleDeclaration, LordsError> {
    Ok(SimpleDeclaration {
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
