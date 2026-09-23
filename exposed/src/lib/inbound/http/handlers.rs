use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::entity_search::{Entity, EntitySearchError, EntitySearchRequest},
        services::entity_search::EntitySearchService,
    },
    inbound::http::{error::ApiError, state::AppState, success::ApiSuccess},
};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SearchEntitiesHttpRequest {
    term: String,
    strictness: Option<f32>,
}

impl SearchEntitiesHttpRequest {
    /// Converts the HTTP request body into a domain request.
    fn try_into_domain(self) -> Result<EntitySearchRequest, EntitySearchError> {
        match self.strictness {
            Some(s) => EntitySearchRequest::new_with_strictness(self.term, s),
            None => EntitySearchRequest::new_strict(self.term),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchEntityResponseData {
    entities: Vec<SearchEntity>,
}

impl From<&[Entity]> for SearchEntityResponseData {
    fn from(value: &[Entity]) -> Self {
        let entities = value.iter().map(SearchEntity::from).collect();
        Self { entities }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchEntity {
    name: String,
    kind: String,
}

impl From<&Entity> for SearchEntity {
    fn from(value: &Entity) -> Self {
        Self {
            name: value.name().to_string(),
            kind: value.kind().to_string(),
        }
    }
}

impl From<EntitySearchError> for ApiError {
    fn from(value: EntitySearchError) -> Self {
        match value {
            EntitySearchError::InvalidTerm(_) => Self::UnprocessibleEntity(value.to_string()),
            EntitySearchError::InvalidStrictness(_) => Self::UnprocessibleEntity(value.to_string()),
            EntitySearchError::UnexpectedError(_) => Self::InternalServerError,
        }
    }
}

pub async fn search_entities<E: EntitySearchService>(
    State(state): State<AppState<E>>,
    Query(query): Query<SearchEntitiesHttpRequest>,
) -> Result<ApiSuccess<SearchEntityResponseData>, ApiError> {
    let domain_req = query.try_into_domain()?;
    state
        .entity_search_service
        .search_entities(&domain_req)
        .await
        .map_err(ApiError::from)
        .map(|ref entities| ApiSuccess::new(StatusCode::OK, entities.as_slice().into()))
}
