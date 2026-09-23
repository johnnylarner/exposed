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
    InternalServerError,
    #[error("unable to process malformed request: {0}")]
    /// Bad request from user
    UnprocessibleEntity(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = match self {
            ApiError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::UnprocessibleEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
        };
        (code, self.to_string()).into_response()
    }
}
