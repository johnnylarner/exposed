use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    domain::{
        models::{lords::LordRegister, parliament_member::MemberId},
        services::{
            entity_details::EntityDetailsService, entity_search::EntitySearchService,
            rankings::RankingsService, updates::UpdatesService,
        },
    },
    inbound::http::{error::ApiError, state::AppState, success::ApiSuccess},
};

pub async fn lord_details<
    E: EntitySearchService,
    D: EntityDetailsService,
    R: RankingsService,
    U: UpdatesService,
>(
    State(state): State<AppState<E, D, R, U>>,
    Path(id): Path<String>,
) -> Result<ApiSuccess<LordRegister>, ApiError> {
    let id: MemberId = id
        .parse()
        .map_err(|_| ApiError::UnprocessibleEntity("Invalid Lords member ID".into()))?;
    let register = state.entity_details_service.lord(id).await?;
    Ok(ApiSuccess::new(StatusCode::OK, register))
}
