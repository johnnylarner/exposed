use super::entity_details::DeclaredGbpPeriodResponse;
use crate::{
    domain::{
        models::rankings::{Ranking, Rankings},
        services::{
            entity_details::EntityDetailsService, entity_search::EntitySearchService,
            rankings::RankingsService, updates::UpdatesService,
        },
    },
    inbound::http::{error::ApiError, state::AppState, success::ApiSuccess},
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Serialize;
#[derive(Serialize)]
pub struct RankingResponse {
    id: Option<String>,
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
    total: DeclaredGbpPeriodResponse,
}
#[derive(Serialize)]
pub struct RankingsResponse {
    period: crate::domain::models::reporting_period::ReportingPeriod,
    current_term_start: chrono::DateTime<chrono::Utc>,
    members: Vec<RankingResponse>,
    individuals: Vec<RankingResponse>,
    companies: Vec<RankingResponse>,
    parties: Vec<RankingResponse>,
}
fn response<I>(row: Ranking<I>, identity: impl FnOnce(I) -> (String, String)) -> RankingResponse {
    let (id, name) = identity(row.identity);
    RankingResponse {
        id: Some(id),
        name: Some(name),
        context: None,
        total: row.total.into(),
    }
}
impl From<Rankings> for RankingsResponse {
    fn from(rankings: Rankings) -> Self {
        Self {
            period: rankings.period,
            current_term_start: rankings.current_term_start,
            members: rankings
                .members
                .into_iter()
                .map(|row| {
                    let context = [
                        row.identity.party_name.as_deref(),
                        row.identity.membership_from.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · ");
                    let mut result = response(row, |identity| {
                        (identity.id.value().to_string(), identity.name)
                    });
                    result.context = (!context.is_empty()).then_some(context);
                    result
                })
                .collect(),
            individuals: rankings
                .individuals
                .into_iter()
                .map(|row| {
                    response(row, |identity| {
                        (identity.id.value().to_string(), identity.name)
                    })
                })
                .collect(),
            companies: rankings
                .companies
                .into_iter()
                .map(|row| {
                    response(row, |identity| {
                        (identity.id.value().to_string(), identity.name)
                    })
                })
                .collect(),
            parties: rankings
                .parties
                .into_iter()
                .map(|row| RankingResponse {
                    id: row.identity.id.map(|id| id.to_string()),
                    name: row.identity.name,
                    context: None,
                    total: row.total.into(),
                })
                .collect(),
        }
    }
}
pub async fn rankings<
    E: EntitySearchService,
    D: EntityDetailsService,
    R: RankingsService,
    U: UpdatesService,
>(
    State(state): State<AppState<E, D, R, U>>,
    Query(query): Query<super::entity_details::PeriodQuery>,
) -> Result<ApiSuccess<RankingsResponse>, ApiError> {
    let rankings = state
        .rankings_service
        .rankings(query.period)
        .await
        .map_err(|error| {
            tracing::error!(?error, "rankings failed");
            ApiError::InternalServerError
        })?;
    Ok(ApiSuccess::new(StatusCode::OK, rankings.into()))
}
