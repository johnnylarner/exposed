use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Generic success message
pub struct ApiSuccess<S: Serialize> {
    code: StatusCode,
    response: S,
}

impl<S: Serialize> ApiSuccess<S> {
    /// Creates new instance
    pub fn new(code: StatusCode, response: S) -> Self {
        Self { code, response }
    }
}

impl<S: Serialize> IntoResponse for ApiSuccess<S> {
    fn into_response(self) -> Response {
        (self.code, Json(self.response)).into_response()
    }
}
