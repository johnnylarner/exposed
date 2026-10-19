use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

const INTERNAL_ERROR_MESSAGE: &str = "internal server error";

#[derive(Error, Debug)]
/// Errors communicated by the HTTP API
pub enum ApiError {
    #[error("{INTERNAL_ERROR_MESSAGE}")]
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
        match self {
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            Self::UnprocessibleEntity(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
            }
            Self::InternalServerError(error) => {
                let mut response = internal_error_response(StatusCode::INTERNAL_SERVER_ERROR);
                response
                    .extensions_mut()
                    .insert(InternalFailure(format!("{error:#}")));
                response
            }
        }
    }
}

#[derive(Clone)]
pub(super) struct InternalFailure(pub(super) String);

pub(super) fn internal_error_response(status: StatusCode) -> Response {
    (status, INTERNAL_ERROR_MESSAGE).into_response()
}
