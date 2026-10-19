#[path = "common/tracing.rs"]
mod capture;

use std::sync::Arc;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use exposed::{
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
    inbound::http::{routes::routes, state::AppState},
};
use tower::ServiceExt;
use tracing::instrument::WithSubscriber;
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt};

#[derive(Clone)]
struct Services {
    fail: bool,
}

impl EntitySearchService for Services {
    async fn search_entities(
        &self,
        _: &EntitySearchRequest,
    ) -> Result<Vec<Entity>, EntitySearchError> {
        if self.fail {
            Err(EntitySearchError::UnexpectedError(
                "private search cause".into(),
            ))
        } else {
            Ok(Vec::new())
        }
    }
}

impl EntityDetailsService for Services {
    async fn member(&self, _: MemberId) -> Result<MemberDetails, EntityDetailsError> {
        Err(self.details_error())
    }
    async fn funder(&self, _: FunderId) -> Result<FunderDetails, EntityDetailsError> {
        Err(self.details_error())
    }
}

impl Services {
    fn details_error(&self) -> EntityDetailsError {
        if self.fail {
            EntityDetailsError::Unexpected(EntityDetailsRepoError::DatabaseError(
                "private details cause".into(),
            ))
        } else {
            EntityDetailsError::NotFound
        }
    }
}

#[tokio::test]
async fn every_failed_router_response_has_one_event_and_private_causes_stay_private() {
    for (method, uri, fail, status, route, cause) in [
        (
            "GET",
            "/search?term=John&max_entries=10",
            true,
            500,
            "/search",
            "private search cause",
        ),
        (
            "GET",
            "/members/1",
            true,
            500,
            "/members/{id}",
            "private details cause",
        ),
        (
            "GET",
            "/funders/00000000-0000-0000-0000-000000000001",
            true,
            500,
            "/funders/{id}",
            "private details cause",
        ),
        ("GET", "/members/1", false, 404, "/members/{id}", ""),
        (
            "GET",
            "/search?term=John&max_entries=0",
            false,
            422,
            "/search",
            "",
        ),
        (
            "GET",
            "/search?term=secret&max_entries=bad",
            false,
            400,
            "/search",
            "",
        ),
        ("GET", "/members/not-an-id", false, 422, "/members/{id}", ""),
        (
            "GET",
            "/unknown-secret?token=secret",
            false,
            404,
            "<unmatched>",
            "",
        ),
        ("POST", "/healthz", false, 405, "/healthz", ""),
        ("GET", "/healthz", false, 204, "/healthz", ""),
        (
            "GET",
            "/search?term=John&max_entries=10",
            false,
            200,
            "/search",
            "",
        ),
    ] {
        let capture = capture::Capture::default();
        let services = Arc::new(Services { fail });
        let router = routes().with_state(AppState {
            entity_search_service: services.clone(),
            entity_details_service: services,
        });
        let response = router
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .with_subscriber(capture.subscriber())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::from_u16(status).unwrap(),
            "{uri}"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let errors = capture.errors();
        assert_eq!(
            errors.len(),
            usize::from(status >= 400),
            "{uri}: {errors:?}"
        );
        let events = capture.events();
        let event = events.last().unwrap();
        assert_eq!(event["fields"]["status"], status);
        assert_eq!(event["span"]["route"], route);
        assert_eq!(event["span"]["method"], method);
        assert!(event["fields"]["duration_ms"].is_number());
        assert!(!serde_json::to_string(&events).unwrap().contains("secret"));
        if !cause.is_empty() {
            assert!(
                errors[0]["fields"]["error"]
                    .as_str()
                    .unwrap()
                    .contains(cause)
            );
            assert_eq!(&body[..], b"internal server error");
        }
    }
}

#[tokio::test]
async fn error_only_filter_retains_request_context() {
    let capture = capture::Capture::default();
    let services = Arc::new(Services { fail: true });
    let router = routes().with_state(AppState {
        entity_search_service: services.clone(),
        entity_details_service: services,
    });
    let response = router
        .oneshot(
            Request::builder()
                .uri("/members/1")
                .body(Body::empty())
                .unwrap(),
        )
        .with_subscriber(capture.subscriber().with(LevelFilter::ERROR))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let errors = capture.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["span"]["method"], "GET");
    assert_eq!(errors[0]["span"]["route"], "/members/{id}");
}
