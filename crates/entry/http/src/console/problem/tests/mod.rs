#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use std::io;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, header};
use axum::response::IntoResponse;
use axum::routing::get;
use crane_ports::settings::{Invalid, InvalidKey, OnboardingError};
use http_body_util::BodyExt;
use serde_json::Value;
use tokio::runtime::Builder;
use tower::ServiceExt;

use super::*;
use crate::context::{TrustedProxies, request_context};

type Make = fn() -> Problem;

struct Rendered {
    status: u16,
    body: Value,
    retry_after: Option<String>,
}

/// Проблема проходит тот же внешний слой, что и в сервере: он рендерит тело и ставит `Retry-After`.
fn render(make: Make) -> Rendered {
    let router = Router::new()
        .route("/", get(move || async move { make().into_response() }))
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(TrustedProxies::default()),
            request_context,
        ));
    let response = Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(router.oneshot(Request::builder().uri("/").body(Body::empty()).unwrap()))
        .unwrap();
    let (parts, body) = response.into_parts();
    let bytes = Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(body.collect())
        .unwrap()
        .to_bytes();
    Rendered {
        status: parts.status.as_u16(),
        body: serde_json::from_slice(&bytes).unwrap(),
        retry_after: parts
            .headers
            .get(header::RETRY_AFTER)
            .map(|value| value.to_str().unwrap().to_owned()),
    }
}

fn unreachable_setting() -> SettingError {
    SettingError::Unreachable(Box::new(io::Error::other("db.internal:5432 refused")))
}

fn invalid() -> SettingError {
    SettingError::Invalid(Invalid {
        errors: vec![InvalidKey {
            key: "max_publish_bytes".to_owned(),
            detail: "ожидается число".to_owned(),
        }],
    })
}

fn about_blank() -> String {
    "about:blank".to_owned()
}

fn urn(slug: &str) -> String {
    format!("urn:crane:problem:{slug}")
}

#[test]
fn every_setting_error_maps_to_status_problem_type_and_retry_after() {
    // Arrange
    let cases: [Make; 8] = [
        || SettingError::Unknown.into(),
        || {
            SettingError::Locked {
                var: "CRANE_BLOBS".to_owned(),
            }
            .into()
        },
        || SettingError::NotPersistent.into(),
        || invalid().into(),
        || SettingError::Conflict.into(),
        || SettingError::SecretsUnavailable.into(),
        || unreachable_setting().into(),
        || SettingError::Backend(Box::new(io::Error::other("pg: relation missing"))).into(),
    ];

    // Act
    let actual = cases.map(|make| {
        let rendered = render(make);
        (
            rendered.status,
            rendered.body["type"].as_str().unwrap().to_owned(),
            rendered.retry_after,
        )
    });

    // Assert
    assert_eq!(
        actual,
        [
            (404, about_blank(), None),
            (409, urn("setting-locked"), None),
            (409, urn("settings-not-persistent"), None),
            (422, urn("setting-invalid"), None),
            (412, about_blank(), None),
            (422, urn("secrets-unavailable"), None),
            (503, urn("settings-unreachable"), Some("10".to_owned())),
            (500, about_blank(), None),
        ]
    );
}

#[test]
fn unreachable_onboarding_store_maps_like_unreachable_settings_store() {
    // Arrange
    let cases: [Make; 2] = [
        || {
            unreachable(&OnboardingError::Unreachable(Box::new(io::Error::other(
                "x",
            ))))
        },
        || unreachable_setting().into(),
    ];

    // Act
    let actual = cases.map(|make| {
        let rendered = render(make);
        (
            rendered.status,
            rendered.body["type"].clone(),
            rendered.retry_after,
        )
    });

    // Assert
    assert_eq!(actual[0], actual[1]);
    assert_eq!(actual[0].2.as_deref(), Some("10"));
}

#[test]
fn locked_problem_names_the_environment_variable() {
    // Arrange
    let make: Make = || {
        SettingError::Locked {
            var: "CRANE_BLOBS".to_owned(),
        }
        .into()
    };

    // Act
    let rendered = render(make);

    // Assert
    assert_eq!(rendered.body["var"], "CRANE_BLOBS");
}

#[test]
fn invalid_problem_lists_every_failed_key() {
    // Arrange
    let make: Make = || invalid().into();

    // Act
    let rendered = render(make);

    // Assert
    assert_eq!(
        rendered.body["errors"],
        serde_json::json!([{"key": "max_publish_bytes", "detail": "ожидается число"}])
    );
}

#[test]
fn infrastructure_failures_do_not_leak_their_source_to_the_client() {
    // Arrange
    let cases: [Make; 2] = [
        || unreachable_setting().into(),
        || SettingError::Backend(Box::new(io::Error::other("db.internal:5432 refused"))).into(),
    ];

    // Act
    let bodies = cases.map(|make| render(make).body.to_string());

    // Assert
    assert_eq!(
        bodies.map(|body| body.contains("db.internal")),
        [false, false]
    );
}
