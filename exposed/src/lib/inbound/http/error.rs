use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
/// Errors communicated by the HTTP API
pub enum ApiError {
    #[error("internal server error")]
    /// Private error, message should only be logged
    InternalServerError(#[source] anyhow::Error),
    /// Entity identity absent from the store.
    #[error("entity not found")]
    NotFound,
    #[error("unable to process malformed request: {0}")]
    /// Bad request from user
    UnprocessibleEntity(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::UnprocessibleEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
        };
        let mut response = (code, self.to_string()).into_response();
        response
            .extensions_mut()
            .insert(ResponseFailure(Arc::new(self)));
        response
    }
}

#[derive(Clone, Debug)]
pub(super) struct ResponseFailure(Arc<ApiError>);

impl ResponseFailure {
    pub(super) fn diagnostic(&self) -> String {
        match self.0.as_ref() {
            ApiError::InternalServerError(source) => format!("{source:#}"),
            ApiError::NotFound => "entity not found".into(),
            ApiError::UnprocessibleEntity(_) => "request validation failed".into(),
        }
    }

    pub(super) fn category(&self) -> &'static str {
        match self.0.as_ref() {
            ApiError::NotFound => "entity.not_found",
            ApiError::UnprocessibleEntity(_) => "request.validation",
            ApiError::InternalServerError(source) => {
                use crate::domain::{
                    models::entity_search::EntitySearchError,
                    services::entity_details::EntityDetailsError,
                };
                if source.downcast_ref::<EntitySearchError>().is_some() {
                    "entity_search.unexpected"
                } else if source.downcast_ref::<EntityDetailsError>().is_some() {
                    "entity_details.database"
                } else {
                    "service.unexpected"
                }
            }
        }
    }
}
