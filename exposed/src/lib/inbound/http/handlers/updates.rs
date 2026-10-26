use crate::{
    domain::{
        models::updates::{EntityUpdate, MonthlyUpdates},
        services::{
            entity_details::EntityDetailsService, entity_search::EntitySearchService,
            rankings::RankingsService, updates::UpdatesService,
        },
    },
    inbound::http::{error::ApiError, state::AppState, success::ApiSuccess},
};
use axum::{extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::Serialize;
#[derive(Serialize)]
pub struct UpdateResponse {
    id: String,
    name: String,
    declaration_count: usize,
    latest_registration: DateTime<Utc>,
}
#[derive(Serialize)]
pub struct UpdatesResponse {
    month_start: DateTime<Utc>,
    month_end: DateTime<Utc>,
    members: Vec<UpdateResponse>,
    companies: Vec<UpdateResponse>,
}
fn response<I>(
    row: EntityUpdate<I>,
    identity: impl FnOnce(I) -> (String, String),
) -> UpdateResponse {
    let (id, name) = identity(row.identity);
    UpdateResponse {
        id,
        name,
        declaration_count: row.declaration_count.get(),
        latest_registration: row.latest_registration,
    }
}
impl From<MonthlyUpdates> for UpdatesResponse {
    fn from(updates: MonthlyUpdates) -> Self {
        Self {
            month_start: updates.month.start(),
            month_end: updates.month.end(),
            members: updates
                .members
                .into_iter()
                .map(|row| {
                    response(row, |identity| {
                        (identity.id.value().to_string(), identity.name)
                    })
                })
                .collect(),
            companies: updates
                .companies
                .into_iter()
                .map(|row| {
                    response(row, |identity| {
                        (identity.id.value().to_string(), identity.name)
                    })
                })
                .collect(),
        }
    }
}
pub async fn updates<
    E: EntitySearchService,
    D: EntityDetailsService,
    R: RankingsService,
    U: UpdatesService,
>(
    State(state): State<AppState<E, D, R, U>>,
) -> Result<ApiSuccess<UpdatesResponse>, ApiError> {
    let now = Utc::now();
    let updates = state.updates_service.updates(now).await.map_err(|error| {
        tracing::error!(?error, "updates failed");
        ApiError::InternalServerError
    })?;
    Ok(ApiSuccess::new(StatusCode::OK, updates.into()))
}
