use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
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
        let code = match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::UnprocessibleEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
        };
        let mut response = (code, self.to_string()).into_response();
        if let Self::InternalServerError(error) = self {
            response
                .extensions_mut()
                .insert(InternalFailure(format!("{error:#}")));
        }
        response
    }
}

#[derive(Clone)]
pub(super) struct InternalFailure(pub(super) String);
