use crate::{
    domain::{
        models::{
            entity_details::{
                CurrencyBreakdown, DeclaredFunding, FunderDetails, FunderProfile, FunderReference,
                MemberDetails, MemberProfile, PartyTotal, RecentDeclaration, RecipientAllocation,
            },
            funder::FunderId,
            parliament_member::MemberId,
        },
        services::{
            entity_details::{EntityDetailsError, EntityDetailsService},
            entity_search::EntitySearchService,
        },
    },
    inbound::http::{error::ApiError, state::AppState, success::ApiSuccess},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

impl From<EntityDetailsError> for ApiError {
    fn from(error: EntityDetailsError) -> Self {
        match error {
            EntityDetailsError::NotFound => Self::NotFound,
            EntityDetailsError::Unexpected(_) => Self::InternalServerError,
        }
    }
}

#[derive(Serialize)]
pub struct MemberResponse {
    id: String,
    name: String,
    party_id: i32,
    party_name: String,
    membership_from: String,
    is_current_commons: bool,
}
impl From<MemberProfile> for MemberResponse {
    fn from(member: MemberProfile) -> Self {
        Self {
            id: member.id.value().to_string(),
            name: member.name,
            party_id: member.party_id,
            party_name: member.party_name,
            membership_from: member.membership_from,
            is_current_commons: member.is_current_commons,
        }
    }
}

#[derive(Serialize)]
pub struct FunderReferenceResponse {
    id: String,
    name: String,
}
impl From<FunderReference> for FunderReferenceResponse {
    fn from(funder: FunderReference) -> Self {
        Self {
            id: funder.id.value().to_string(),
            name: funder.name,
        }
    }
}

#[derive(Serialize)]
pub struct FundingResponse {
    funder: Option<FunderReferenceResponse>,
    amount: Option<String>,
    currency: Option<String>,
    payment_type: Option<String>,
}
impl From<DeclaredFunding> for FundingResponse {
    fn from(entry: DeclaredFunding) -> Self {
        Self {
            funder: entry.funder.map(Into::into),
            amount: entry
                .amount
                .map(|amount| amount.normalized().to_plain_string()),
            currency: entry.currency,
            payment_type: entry.payment_type,
        }
    }
}

#[derive(Serialize)]
pub struct DeclarationResponse {
    source_id: i32,
    category_name: String,
    registered_at: Option<DateTime<Utc>>,
    entries: Vec<FundingResponse>,
}
impl From<RecentDeclaration> for DeclarationResponse {
    fn from(declaration: RecentDeclaration) -> Self {
        Self {
            source_id: declaration.source_id,
            category_name: declaration.category_name,
            registered_at: declaration.registered_at,
            entries: declaration.entries.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct MemberDetailsResponse {
    member: MemberResponse,
    declarations: Vec<DeclarationResponse>,
    declaration_count: i64,
    declaration_limit: usize,
}
impl From<MemberDetails> for MemberDetailsResponse {
    fn from(details: MemberDetails) -> Self {
        Self {
            member: details.member.into(),
            declarations: details.declarations.into_iter().map(Into::into).collect(),
            declaration_count: details.declaration_count,
            declaration_limit: details.declaration_limit,
        }
    }
}

#[derive(Serialize)]
pub struct FunderResponse {
    id: String,
    name: String,
    funder_kind: String,
    company_number: Option<String>,
    aliases: Vec<String>,
}
impl From<FunderProfile> for FunderResponse {
    fn from(profile: FunderProfile) -> Self {
        Self {
            id: profile.funder.id().value().to_string(),
            name: profile.funder.name().to_string(),
            funder_kind: profile.funder.kind().to_string(),
            company_number: profile.company_number,
            aliases: profile.aliases,
        }
    }
}

#[derive(Serialize)]
pub struct PartyTotalResponse {
    party_id: i32,
    party_name: String,
    amount: Option<String>,
    entry_count: i64,
    unknown_amount_count: i64,
}
impl From<PartyTotal> for PartyTotalResponse {
    fn from(party: PartyTotal) -> Self {
        Self {
            party_id: party.party_id,
            party_name: party.party_name,
            amount: party
                .amount
                .map(|amount| amount.normalized().to_plain_string()),
            entry_count: party.entry_count,
            unknown_amount_count: party.unknown_amount_count,
        }
    }
}

#[derive(Serialize)]
pub struct RecipientTotalResponse {
    member: MemberResponse,
    amount: Option<String>,
    entry_count: i64,
    unknown_amount_count: i64,
}
impl From<RecipientAllocation> for RecipientTotalResponse {
    fn from(recipient: RecipientAllocation) -> Self {
        Self {
            member: recipient.member.into(),
            amount: recipient
                .known_total
                .map(|amount| amount.normalized().to_plain_string()),
            entry_count: recipient.entry_count,
            unknown_amount_count: recipient.unknown_amount_count,
        }
    }
}

#[derive(Serialize)]
pub struct CurrencyResponse {
    currency: String,
    parties: Vec<PartyTotalResponse>,
    top_recipients: Vec<RecipientTotalResponse>,
}
impl From<CurrencyBreakdown> for CurrencyResponse {
    fn from(group: CurrencyBreakdown) -> Self {
        Self {
            currency: group.currency,
            parties: group.parties.into_iter().map(Into::into).collect(),
            top_recipients: group.top_recipients.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct FunderDetailsResponse {
    funder: FunderResponse,
    currencies: Vec<CurrencyResponse>,
    entry_count: i64,
    unknown_currency_count: i64,
    recipient_limit: usize,
}
impl From<FunderDetails> for FunderDetailsResponse {
    fn from(details: FunderDetails) -> Self {
        Self {
            funder: details.profile.into(),
            currencies: details.currencies.into_iter().map(Into::into).collect(),
            entry_count: details.entry_count,
            unknown_currency_count: details.unknown_currency_count,
            recipient_limit: details.recipient_limit,
        }
    }
}

pub async fn member_details<E: EntitySearchService, D: EntityDetailsService>(
    State(state): State<AppState<E, D>>,
    Path(id): Path<String>,
) -> Result<ApiSuccess<MemberDetailsResponse>, ApiError> {
    let id = id
        .parse::<MemberId>()
        .map_err(|error| ApiError::UnprocessibleEntity(error.to_string()))?;
    let details = state.entity_details_service.member(id).await?;
    Ok(ApiSuccess::new(StatusCode::OK, details.into()))
}

pub async fn funder_details<E: EntitySearchService, D: EntityDetailsService>(
    State(state): State<AppState<E, D>>,
    Path(id): Path<String>,
) -> Result<ApiSuccess<FunderDetailsResponse>, ApiError> {
    let id = id
        .parse::<FunderId>()
        .map_err(|_| ApiError::UnprocessibleEntity("Invalid funder ID".into()))?;
    let details = state.entity_details_service.funder(id).await?;
    Ok(ApiSuccess::new(StatusCode::OK, details.into()))
}
