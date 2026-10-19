use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use super::error::ApiError;

/// Generic success message
pub struct ApiSuccess<S: Serialize> {
    code: StatusCode,
    response: S,
}

impl<S: Serialize> ApiSuccess<S> {
    /// Creates new instance
    pub const fn new(code: StatusCode, response: S) -> Self {
        Self { code, response }
    }
}

impl<S: Serialize> IntoResponse for ApiSuccess<S> {
    fn into_response(self) -> Response {
        match serde_json::to_vec(&self.response) {
            Ok(body) => (
                self.code,
                [(header::CONTENT_TYPE, "application/json")],
                body,
            )
                .into_response(),
            Err(error) => ApiError::InternalServerError(error.into()).into_response(),
        }
    }
}
