use std::time::Instant;

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use tracing::Instrument;

use super::error::InternalFailure;

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
        response
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
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
    };
    use serde::{Serialize, Serializer};
    use tower::ServiceExt;
    use tracing::instrument::WithSubscriber;

    struct Broken;
    impl Serialize for Broken {
        fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("serialization failed"))
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
        let errors = capture.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["fields"]["status"], 500);
    }
}
