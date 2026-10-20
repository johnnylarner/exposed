use std::time::{Duration, Instant};

use axum::{
    Router,
    extract::{MatchedPath, Request},
    middleware::{self, Next},
    response::Response,
};
use tracing::{Instrument, Level};
use tracing_subscriber::{EnvFilter, util::SubscriberInitExt};

pub(super) fn init_subscriber() -> anyhow::Result<()> {
    let directives = match std::env::var("RUST_LOG") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => "warn,exposed=info".into(),
        Err(error) => return Err(error.into()),
    };
    let filter = EnvFilter::builder().with_regex(false).parse(directives)?;
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .finish()
        .try_init()?;
    Ok(())
}

pub(super) fn instrument<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(middleware::from_fn(trace_request))
}

async fn trace_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let route = request.extensions().get::<MatchedPath>().cloned();
    let health = route
        .as_ref()
        .is_some_and(|path| path.as_str() == "/healthz");
    let route_name = route.as_ref().map(MatchedPath::as_str);
    let span = if health {
        tracing::span!(Level::DEBUG, "http.request",
            http.request.method = %method, http.route = route_name)
    } else {
        tracing::span!(Level::INFO, "http.request",
            http.request.method = %method, http.route = route_name)
    };
    async move {
        let started = Instant::now();
        let response = next.run(request).await;
        complete(
            &response,
            started.elapsed(),
            &method,
            route.as_ref(),
            health,
        );
        response
    }
    .instrument(span)
    .await
}

fn complete(
    response: &Response,
    latency: Duration,
    method: &axum::http::Method,
    route: Option<&MatchedPath>,
    health: bool,
) {
    let status = response.status();
    let category = if status.is_server_error() {
        Some("http.server_error")
    } else if status.is_client_error() {
        Some("http.client_error")
    } else {
        None
    };
    let route = route.map(MatchedPath::as_str);
    macro_rules! emit {
        ($level:ident) => {
            tracing::$level!(
                name: "http.request.completed",
                name = "http.request.completed",
                http.request.method = %method,
                http.route = route,
                http.response.status_code = u64::from(status.as_u16()),
                duration_ms = latency.as_secs_f64() * 1000.0,
                error.type = category,
            )
        };
    }
    if health && status.is_success() {
        emit!(debug);
    } else {
        emit!(info);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };

    use axum::response::IntoResponse;
    use axum::{
        body::{Body, to_bytes},
        http::{Method, StatusCode},
        routing::get,
    };
    use serde_json::Value;
    use tower::ServiceExt;
    use tracing::instrument::WithSubscriber;

    use super::*;
    use crate::{
        domain::{
            models::{
                entity_details::{FunderDetails, MemberDetails},
                entity_search::{Entity, EntitySearchError, EntitySearchRequest},
                funder::FunderId,
                parliament_member::MemberId,
            },
            repositories::entity_details::EntityDetailsRepoError,
            services::{
                entity_details::{EntityDetailsError, EntityDetailsService},
                entity_search::EntitySearchService,
            },
        },
        inbound::http::{error::ApiError, routes::routes, state::AppState},
    };

    #[derive(Clone)]
    struct Search;
    impl EntitySearchService for Search {
        async fn search_entities(
            &self,
            _: &EntitySearchRequest,
        ) -> Result<Vec<Entity>, EntitySearchError> {
            tokio::task::yield_now().await;
            Err(EntitySearchError::UnexpectedError(
                "database search failed".into(),
            ))
        }
    }
    #[derive(Clone)]
    struct Details;
    impl EntityDetailsService for Details {
        async fn member(&self, id: MemberId) -> Result<MemberDetails, EntityDetailsError> {
            tokio::task::yield_now().await;
            if id.value() == 2 {
                return Err(
                    EntityDetailsRepoError::DatabaseError("member database failed".into()).into(),
                );
            }
            Err(EntityDetailsError::NotFound)
        }
        async fn funder(&self, _: FunderId) -> Result<FunderDetails, EntityDetailsError> {
            tokio::task::yield_now().await;
            Err(EntityDetailsRepoError::DatabaseError("database details failed".into()).into())
        }
    }

    fn app() -> Router {
        routes().with_state(AppState {
            entity_search_service: Arc::new(Search),
            entity_details_service: Arc::new(Details),
        })
    }

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);
    impl Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Buffer {
        fn events(&self) -> Vec<Value> {
            String::from_utf8(self.0.lock().unwrap().clone())
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        }
        fn subscriber(&self, filter: &str) -> tracing::Dispatch {
            let buffer = self.clone();
            tracing::Dispatch::new(
                tracing_subscriber::fmt()
                    .json()
                    .without_time()
                    .with_env_filter(
                        EnvFilter::builder()
                            .with_regex(false)
                            .parse(filter)
                            .unwrap(),
                    )
                    .with_writer(move || buffer.clone())
                    .finish(),
            )
        }
    }

    async fn request(
        router: Router,
        method: Method,
        uri: &str,
        filter: &str,
    ) -> (Response, Vec<Value>) {
        let buffer = Buffer::default();
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", "PRIVATE_AUTH_HEADER")
            .body(Body::from("PRIVATE_REQUEST_BODY"))
            .unwrap();
        let response = router
            .oneshot(request)
            .with_subscriber(buffer.subscriber(filter))
            .await
            .unwrap();
        let events = buffer.events();
        let output = serde_json::to_string(&events).unwrap();
        for secret in [
            "PRIVATE_AUTH_HEADER",
            "PRIVATE_REQUEST_BODY",
            "PRIVATE_QUERY",
        ] {
            assert!(!output.contains(secret), "leaked {secret}: {output}");
        }
        (response, events)
    }

    #[tokio::test]
    async fn records_one_structured_completion_for_each_response() {
        for (method, uri, status, level, route, category) in [
            (
                Method::GET,
                "/healthz",
                204,
                "DEBUG",
                Some("/healthz"),
                None,
            ),
            (
                Method::GET,
                "/search?term=x&max_entries=1&private=PRIVATE_QUERY",
                422,
                "INFO",
                Some("/search"),
                Some("http.client_error"),
            ),
            (
                Method::GET,
                "/members/1",
                404,
                "INFO",
                Some("/members/{id}"),
                Some("http.client_error"),
            ),
            (
                Method::GET,
                "/search?max_entries=garbage",
                400,
                "INFO",
                Some("/search"),
                Some("http.client_error"),
            ),
            (
                Method::GET,
                "/PRIVATE_QUERY",
                404,
                "INFO",
                None,
                Some("http.client_error"),
            ),
            (
                Method::POST,
                "/healthz",
                405,
                "INFO",
                Some("/healthz"),
                Some("http.client_error"),
            ),
            (
                Method::GET,
                "/search?term=Alice&max_entries=1",
                500,
                "INFO",
                Some("/search"),
                Some("http.server_error"),
            ),
            (
                Method::GET,
                "/funders/00000000-0000-0000-0000-000000000000",
                500,
                "INFO",
                Some("/funders/{id}"),
                Some("http.server_error"),
            ),
        ] {
            let (response, events) = request(app(), method.clone(), uri, "exposed=debug").await;
            assert_eq!(response.status().as_u16(), status, "{uri}");
            if status == 500 {
                assert_eq!(
                    to_bytes(response.into_body(), 1024).await.unwrap(),
                    "internal server error"
                );
            }
            let diagnostic_count = events
                .iter()
                .filter(|event| event["level"] == "ERROR")
                .count();
            let handler_failure = matches!(status, 422 | 500) || uri == "/members/1";
            assert_eq!(
                diagnostic_count,
                usize::from(handler_failure),
                "{uri}: {events:?}"
            );
            let access: Vec<_> = events
                .iter()
                .filter(|event| event["fields"]["name"] == "http.request.completed")
                .collect();
            assert_eq!(access.len(), 1, "{uri}: {events:?}");
            assert_eq!(events.len(), 1 + usize::from(handler_failure));
            assert_eq!(access[0]["level"], level);
            let fields = &access[0]["fields"];
            assert_eq!(fields["name"], "http.request.completed");
            assert_eq!(fields["http.request.method"], method.as_str());
            assert_eq!(fields["http.response.status_code"], status);
            assert!(fields["duration_ms"].is_number());
            assert_eq!(fields.get("http.route").and_then(Value::as_str), route);
            assert_eq!(fields.get("error.type").and_then(Value::as_str), category);
        }
    }

    #[tokio::test]
    async fn typed_errors_are_logged_before_conversion_even_when_spans_are_filtered() {
        for (uri, expected) in [
            ("/search?term=x&max_entries=1", "InvalidTerm(1)"),
            ("/search?term=Alice&max_entries=0", "TooFewEntries"),
            (
                "/search?term=Alice&max_entries=1&strictness=2",
                "InvalidStrictness(2.0)",
            ),
            ("/members/invalid", "MemberIdError"),
            ("/members/0", "MemberIdError"),
            ("/funders/invalid", "Error"),
            ("/members/1", "NotFound"),
            (
                "/members/2",
                "Unexpected(DatabaseError(\"member database failed\"))",
            ),
            (
                "/search?term=Alice&max_entries=1",
                "UnexpectedError(\"database search failed\")",
            ),
            (
                "/funders/00000000-0000-0000-0000-000000000000",
                "Unexpected(DatabaseError(\"database details failed\"))",
            ),
        ] {
            let (_, events) = request(app(), Method::GET, uri, "error").await;
            assert_eq!(events.len(), 1, "{uri}: {events:?}");
            assert_eq!(events[0]["level"], "ERROR");
            let error = events[0]["fields"]["error"].as_str().unwrap();
            assert!(error.contains(expected), "{uri}: {error}");
        }
    }

    #[tokio::test]
    async fn expected_errors_preserve_exact_response_bodies() {
        for (error, status, body) in [
            (
                ApiError::from(EntitySearchError::InvalidTerm(1)),
                StatusCode::UNPROCESSABLE_ENTITY,
                "unable to process malformed request: term must be at last three characters long; received 1",
            ),
            (
                ApiError::from(EntitySearchError::InvalidStrictness(2.0)),
                StatusCode::UNPROCESSABLE_ENTITY,
                "unable to process malformed request: strictness must be between 0 and 1; get 2",
            ),
            (
                ApiError::from(EntitySearchError::TooFewEntries),
                StatusCode::UNPROCESSABLE_ENTITY,
                "unable to process malformed request: at least one entry must be expected to return",
            ),
            (
                ApiError::UnprocessibleEntity("Invalid funder ID".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
                "unable to process malformed request: Invalid funder ID",
            ),
            (
                ApiError::from(EntityDetailsError::NotFound),
                StatusCode::NOT_FOUND,
                "entity not found",
            ),
        ] {
            let response = error.into_response();
            assert_eq!(response.status(), status);
            assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), body);
        }
    }

    #[tokio::test]
    async fn successful_requests_are_info_and_successful_health_is_quiet_by_default() {
        let router = instrument(Router::new().route(
            "/ok",
            get(|| async { axum::Json(serde_json::json!({"ok": true})) }),
        ));
        let (_, events) = request(
            router,
            Method::GET,
            "/ok?private=PRIVATE_QUERY",
            "exposed=info",
        )
        .await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["level"], "INFO");
        let (response, events) = request(app(), Method::GET, "/healthz", "warn,exposed=info").await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn filtered_health_span_does_not_hide_failed_request_identity() {
        let router = instrument(Router::new().route(
            "/healthz",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        ));
        let (_, events) = request(router, Method::GET, "/healthz", "info").await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["level"], "INFO");
        assert_eq!(events[0]["fields"]["http.request.method"], "GET");
        assert_eq!(events[0]["fields"]["http.route"], "/healthz");
        assert_eq!(events[0]["fields"]["error.type"], "http.server_error");
    }

    #[tokio::test]
    async fn concurrent_requests_each_complete_once() {
        let buffer = Buffer::default();
        let dispatch = buffer.subscriber("exposed=debug");
        let requests = (0..12).map(|index| {
            app()
                .oneshot(
                    Request::builder()
                        .uri(if index % 2 == 0 {
                            "/members/1"
                        } else {
                            "/search?term=Alice&max_entries=1"
                        })
                        .body(Body::empty())
                        .unwrap(),
                )
                .with_subscriber(dispatch.clone())
        });
        for response in futures::future::join_all(requests).await {
            assert!(matches!(
                response.unwrap().status(),
                StatusCode::NOT_FOUND | StatusCode::INTERNAL_SERVER_ERROR
            ));
        }
        let events = buffer.events();
        assert_eq!(events.len(), 24);
        assert_eq!(
            events
                .iter()
                .filter(|event| event["level"] == "INFO")
                .count(),
            12
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event["level"] == "ERROR")
                .count(),
            12
        );
        for event in &events {
            if event["level"] == "INFO" {
                assert_eq!(event["span"]["http.route"], event["fields"]["http.route"]);
            } else if event["fields"]["error"] == "NotFound" {
                assert_eq!(event["span"]["http.route"], "/members/{id}");
            } else {
                assert_eq!(event["span"]["http.route"], "/search");
                assert_eq!(
                    event["fields"]["error"],
                    "UnexpectedError(\"database search failed\")"
                );
            }
            assert_eq!(event["span"]["http.request.method"], "GET");
            assert_eq!(event["span"]["name"], "http.request");
            assert_eq!(event["spans"].as_array().unwrap().len(), 1);
        }
    }
}

#[cfg(test)]
mod subscriber_process_tests {
    use super::init_subscriber;

    #[test]
    fn initialization_child() {
        if std::env::var_os("EXPOSED_TEST_SUBSCRIBER_CHILD").is_none() {
            return;
        }
        init_subscriber().unwrap();
        tracing::info!("subscriber ready");
        assert!(
            init_subscriber().is_err(),
            "duplicate installation must fail"
        );
    }

    #[test]
    fn subscriber_uses_default_filter_stderr_and_rejects_duplicate_installation() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "inbound::http::tracing::subscriber_process_tests::initialization_child",
                "--nocapture",
            ])
            .env("EXPOSED_TEST_SUBSCRIBER_CHILD", "1")
            .env_remove("RUST_LOG")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("subscriber ready"));
        let events: Vec<serde_json::Value> = String::from_utf8(output.stderr)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["fields"]["message"], "subscriber ready");
    }
}
