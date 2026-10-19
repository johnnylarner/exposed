use std::time::Instant;

use axum::{
    body::Body,
    extract::{MatchedPath, Request},
    http::Method,
    middleware::Next,
    response::Response,
};
use tracing::Instrument;

use super::error::{InternalFailure, internal_error_response};

pub(super) async fn trace_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("<unmatched>", MatchedPath::as_str)
        .to_owned();
    let span = tracing::error_span!("http.request", %method, %route);
    async move {
        let started = Instant::now();
        let response = next.run(request).await;
        let status = response.status();
        let duration_ms = started.elapsed().as_secs_f64() * 1000.0;
        if status.is_client_error() || status.is_server_error() {
            let error = response
                .extensions()
                .get::<InternalFailure>()
                .map_or_else(|| status.to_string(), |failure| failure.0.clone());
            tracing::error!(status = status.as_u16(), duration_ms, %error, "request failed");
        } else {
            tracing::info!(status = status.as_u16(), duration_ms, "request completed");
        }
        if status.is_server_error() {
            let mut response = internal_error_response(status);
            if method == Method::HEAD {
                *response.body_mut() = Body::empty();
            }
            response
        } else {
            response
        }
    }
    .instrument(span)
    .await
}

#[cfg(test)]
#[path = "../../../../tests/common/tracing.rs"]
mod capture;

#[cfg(test)]
mod tests {
    use super::{capture, trace_request};
    use crate::inbound::http::success::ApiSuccess;
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
    };
    use serde::{Serialize, Serializer};
    use tower::ServiceExt;
    use tracing::instrument::WithSubscriber;

    struct Broken;
    impl Serialize for Broken {
        fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom(
                "private serialization diagnostic",
            ))
        }
    }

    #[tokio::test]
    async fn serialization_failure_is_logged_at_the_request_boundary() {
        let capture = capture::Capture::default();
        let router = Router::new()
            .route(
                "/broken",
                get(|| async { ApiSuccess::new(StatusCode::OK, Broken) }),
            )
            .layer(middleware::from_fn(trace_request));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/broken")
                    .body(Body::empty())
                    .unwrap(),
            )
            .with_subscriber(capture.subscriber())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!format!("{:?}", response.headers()).contains("private serialization diagnostic"));
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body.as_ref(), b"internal server error");
        let errors = capture.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["fields"]["status"], 500);
        assert_eq!(
            errors[0]["fields"]["error"],
            "private serialization diagnostic"
        );
    }

    #[tokio::test]
    async fn raw_server_errors_are_scrubbed_at_the_request_boundary() {
        for status in [
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::SERVICE_UNAVAILABLE,
        ] {
            let capture = capture::Capture::default();
            let router = Router::new()
                .route(
                    "/raw",
                    get(move || async move {
                        let mut response = (
                            status,
                            [("x-private-diagnostic", "private header")],
                            "private body",
                        )
                            .into_response();
                        response
                            .extensions_mut()
                            .insert(String::from("private extension"));
                        response
                    }),
                )
                .layer(middleware::from_fn(trace_request));
            let response = router
                .oneshot(Request::builder().uri("/raw").body(Body::empty()).unwrap())
                .with_subscriber(capture.subscriber())
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert!(!response.headers().contains_key("x-private-diagnostic"));
            assert!(response.extensions().get::<String>().is_none());
            assert!(
                response
                    .extensions()
                    .get::<super::InternalFailure>()
                    .is_none()
            );
            assert_eq!(
                to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .as_ref(),
                b"internal server error"
            );
            let errors = capture.errors();
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0]["fields"]["status"], status.as_u16());
            assert_eq!(errors[0]["fields"]["error"], status.to_string());
        }
    }

    #[tokio::test]
    async fn success_serialization_failure_is_private_without_middleware() {
        let response = ApiSuccess::new(StatusCode::CREATED, Broken).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!format!("{:?}", response.headers()).contains("private serialization diagnostic"));
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .as_ref(),
            b"internal server error"
        );
    }

    #[tokio::test]
    async fn success_keeps_status_and_json_contract() {
        let response =
            ApiSuccess::new(StatusCode::CREATED, serde_json::json!({"message": "hello"}))
                .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()["content-type"], "application/json");
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .as_ref(),
            br#"{"message":"hello"}"#
        );
    }

    #[tokio::test]
    async fn head_server_error_has_no_body() {
        let capture = capture::Capture::default();
        let router = Router::new()
            .route(
                "/broken",
                get(|| async { ApiSuccess::new(StatusCode::OK, Broken) }),
            )
            .layer(middleware::from_fn(trace_request));
        let response = router
            .oneshot(
                Request::builder()
                    .method("HEAD")
                    .uri("/broken")
                    .body(Body::empty())
                    .unwrap(),
            )
            .with_subscriber(capture.subscriber())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .is_empty()
        );
        let errors = capture.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0]["fields"]["error"],
            "private serialization diagnostic"
        );
    }

    #[tokio::test]
    async fn non_server_responses_keep_their_body_headers_and_extensions() {
        for status in [StatusCode::OK, StatusCode::BAD_REQUEST] {
            let capture = capture::Capture::default();
            let router = Router::new()
                .route(
                    "/public",
                    get(move || async move {
                        let mut response = (status, [("x-public", "public header")], "public body")
                            .into_response();
                        response
                            .extensions_mut()
                            .insert(String::from("public extension"));
                        response
                    }),
                )
                .layer(middleware::from_fn(trace_request));
            let response = router
                .oneshot(
                    Request::builder()
                        .uri("/public")
                        .body(Body::empty())
                        .unwrap(),
                )
                .with_subscriber(capture.subscriber())
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()["x-public"], "public header");
            assert_eq!(
                response.extensions().get::<String>().unwrap(),
                "public extension"
            );
            assert_eq!(
                to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .as_ref(),
                b"public body"
            );
            assert_eq!(
                capture.errors().len(),
                usize::from(status.is_client_error())
            );
        }
    }
}
