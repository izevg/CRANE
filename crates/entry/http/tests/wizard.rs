//! Фаза мастера: протоколы отвечают 503 not-configured, пробы admin остаются зелёными без деталей.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use crane_http::{wizard_admin_router, wizard_router};
use http_body_util::BodyExt;
use serde_json::Value;
use tokio::runtime::Builder;
use tower::ServiceExt;

struct Reply {
    status: StatusCode,
    retry_after: Option<String>,
    content_type: Option<String>,
    body: String,
}

fn call(router: &Router, method: Method, uri: &str) -> Reply {
    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
    runtime.block_on(async {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        let (parts, body) = response.into_parts();
        let header = |name: &str| {
            parts
                .headers
                .get(name)
                .map(|value| value.to_str().unwrap().to_owned())
        };
        Reply {
            status: parts.status,
            retry_after: header("retry-after"),
            content_type: header("content-type"),
            body: String::from_utf8(body.collect().await.unwrap().to_bytes().to_vec()).unwrap(),
        }
    })
}

#[test]
fn protocol_paths_answer_503_not_configured_with_retry_after() {
    // Arrange
    let router = wizard_router(Router::new());
    let paths = [
        "/cargo/main/api/v1/crates/new",
        "/npm/registry/left-pad",
        "/cargo/proxy/index/3/a/abc",
        "/",
        "/anything/else",
    ];

    // Act
    let replies = paths.map(|path| call(&router, Method::PUT, path));

    // Assert
    let summary = replies.map(|reply| {
        let body: Value = serde_json::from_str(&reply.body).unwrap();
        (
            reply.status,
            reply.retry_after,
            reply.content_type,
            body["type"].as_str().unwrap().to_owned(),
        )
    });
    let expected = (
        StatusCode::SERVICE_UNAVAILABLE,
        Some("10".to_owned()),
        Some("application/problem+json".to_owned()),
        "urn:crane:problem:not-configured".to_owned(),
    );
    assert_eq!(summary, std::array::from_fn(|_| expected.clone()));
}

#[test]
fn wizard_routes_are_served_and_only_the_rest_is_not_configured() {
    // Arrange
    let wizard = Router::new().route("/api/v1/onboarding", get(|| async { "state" }));
    let router = wizard_router(wizard);

    // Act
    let replies = [
        call(&router, Method::GET, "/api/v1/onboarding"),
        call(&router, Method::GET, "/cargo/main/x"),
    ];

    // Assert
    let seen = replies.map(|reply| (reply.status, reply.body.contains("not-configured")));
    assert_eq!(
        seen,
        [
            (StatusCode::OK, false),
            (StatusCode::SERVICE_UNAVAILABLE, true)
        ]
    );
}

#[test]
fn admin_probes_stay_green_in_wizard_phase_and_say_nothing_about_configuration() {
    // Arrange
    let router = wizard_admin_router();

    // Act
    let replies = ["/healthz/live", "/healthz/ready"].map(|path| call(&router, Method::GET, path));

    // Assert
    let seen = replies.map(|reply| (reply.status, reply.body));
    assert_eq!(
        seen,
        [
            (StatusCode::OK, "ok".to_owned()),
            (StatusCode::OK, "ok".to_owned())
        ]
    );
}

#[test]
fn admin_router_of_wizard_phase_has_no_other_routes() {
    // Arrange
    let router = wizard_admin_router();

    // Act
    let reply = call(&router, Method::GET, "/metrics");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}
