use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::{error::Error as _, sync::Arc};
use thiserror::Error;

use crate::domain::{
    models::entity_search::EntitySearchError, services::entity_details::EntityDetailsError,
};

/// Errors communicated by the HTTP API.
#[derive(Error, Debug)]
pub enum ApiError {
    /// Search service failure.
    #[error("entity search failed")]
    Search(#[from] EntitySearchError),
    /// Entity detail service failure.
    #[error("entity details failed")]
    Details(#[from] EntityDetailsError),
    /// Bad request from user.
    #[error("unable to process malformed request: {0}")]
    UnprocessibleEntity(String),
}

impl ApiError {
    const fn classification(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Search(EntitySearchError::UnexpectedError(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "entity_search.unexpected",
            ),
            Self::Details(EntityDetailsError::Unexpected(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "entity_details.database")
            }
            Self::Details(EntityDetailsError::NotFound) => {
                (StatusCode::NOT_FOUND, "entity.not_found")
            }
            Self::Search(
                EntitySearchError::InvalidTerm(_)
                | EntitySearchError::InvalidStrictness(_)
                | EntitySearchError::TooFewEntries,
            )
            | Self::UnprocessibleEntity(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "request.validation")
            }
        }
    }

    fn response_body(&self, status: StatusCode) -> String {
        match status {
            StatusCode::INTERNAL_SERVER_ERROR => "internal server error".into(),
            StatusCode::NOT_FOUND => "entity not found".into(),
            _ => match self {
                Self::Search(error) => format!("unable to process malformed request: {error}"),
                Self::Details(_) | Self::UnprocessibleEntity(_) => self.to_string(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, _) = self.classification();
        let mut response = (status, self.response_body(status)).into_response();
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
        let mut diagnostic = self.0.to_string();
        let mut source = self.0.source();
        while let Some(error) = source {
            use std::fmt::Write;
            write!(diagnostic, ": {error}").expect("writing to a string cannot fail");
            source = error.source();
        }
        diagnostic
    }

    pub(super) fn category(&self) -> &'static str {
        self.0.classification().1
    }
}
