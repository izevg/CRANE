//! Служебные точки и токены: whoami, ping, audit, отказ без токена.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::Api;

use super::packument_uri;

#[test]
fn whoami_ping_and_audit_answer_without_touching_storage() {
    // Arrange
    let api = Api::new();

    // Act
    let replies = [
        api.npm_get("/npm/web/-/whoami"),
        api.npm_get("/npm/web/-/ping"),
        api.npm(
            Method::POST,
            "/npm/web/-/npm/v1/security/advisories/bulk",
            b"{}".to_vec(),
        ),
    ];

    // Assert
    assert_eq!(replies[0].json(), json!({"username": "crane"}));
    assert_eq!(replies[1].json(), json!({}));
    assert_eq!(replies[2].status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(
        replies[2].npm_error(),
        "аудит уязвимостей через реестр не поддерживается"
    );
}

#[test]
fn missing_bearer_token_is_unauthorized_with_bearer_challenge() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.with_headers(
        Method::GET,
        &packument_uri("@acme/left-pad"),
        &[],
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.header("www-authenticate"), "Bearer realm=\"crane\"");
    assert_eq!(reply.npm_error(), "требуется токен");
}

#[test]
fn wrong_or_schemeless_token_is_forbidden_and_audited() {
    // Arrange
    let api = Api::new();
    let schemeless = crate::common::TOKEN;
    let cases = ["Bearer wrong", schemeless, "Basic abc"];

    // Act
    let statuses = cases.map(|value| {
        api.with_headers(
            Method::GET,
            &packument_uri("@acme/left-pad"),
            &[("authorization", value)],
            Vec::new(),
        )
        .status
    });

    // Assert
    assert_eq!(statuses, [StatusCode::FORBIDDEN; 3]);
    assert_eq!(api.audit_events().len(), 3);
}
