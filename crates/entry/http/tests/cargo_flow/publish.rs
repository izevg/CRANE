//! Публикация crate: ответ, конфликты, лимит тела, написания имени.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{Api, REPO};

use super::{CRATE, url};

fn publish_ok() -> serde_json::Value {
    json!({ "warnings": { "invalid_categories": [], "invalid_badges": [], "other": [] } })
}

#[test]
fn publish_answers_with_cargo_warnings_object() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.publish(REPO, "demo-crate", "1.2.3", CRATE);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(reply.json(), publish_ok());
}

#[test]
fn republishing_same_version_is_conflict_in_cargo_shape() {
    // Arrange
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Act
    let again = api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Assert
    assert_eq!(again.status, StatusCode::CONFLICT);
    assert_eq!(again.header("content-type"), "application/json");
    assert_eq!(again.detail(), "версия уже существует");
}

#[test]
fn publish_to_proxy_repository_is_method_not_allowed() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.publish("mirror", "demo-crate", "1.0.0", CRATE);

    // Assert
    assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn publish_to_unknown_repository_is_not_found() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.publish("nope", "demo-crate", "1.0.0", CRATE);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[test]
fn truncated_publish_body_is_bad_request() {
    // Arrange
    let api = Api::new();
    let mut body = crate::common::cargo::publish_body("demo-crate", "1.0.0", CRATE);
    body.truncate(body.len() - 3);

    // Act
    let reply = api.authed(Method::PUT, &url("new"), body);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.detail(), "тело короче заявленных длин");
}

#[test]
fn publish_body_over_limit_is_payload_too_large_in_cargo_shape() {
    // Arrange
    let api = Api::new();
    let big = vec![0_u8; crate::common::MAX_PUBLISH_BYTES + 1];

    // Act
    let reply = api.publish(REPO, "demo-crate", "1.0.0", &big);

    // Assert
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_ne!(reply.detail(), "");
}

#[test]
fn publish_body_at_limit_is_accepted() {
    // Arrange
    let api = Api::new();
    let overhead = crate::common::cargo::publish_body("demo-crate", "1.0.0", &[]).len();
    let fits = vec![0_u8; crate::common::MAX_PUBLISH_BYTES - overhead];

    // Act
    let reply = api.publish(REPO, "demo-crate", "1.0.0", &fits);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}

#[test]
fn publishing_name_with_other_spelling_is_rejected() {
    // Arrange
    let api = Api::new();
    api.publish(REPO, "foo-bar", "1.0.0", CRATE);

    // Act
    let reply = api.publish(REPO, "foo_bar", "2.0.0", CRATE);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.detail(), "крейт уже опубликован под именем `foo-bar`");
}

#[test]
fn publishing_new_version_under_same_spelling_is_accepted() {
    // Arrange
    let api = Api::new();
    api.publish(REPO, "foo-bar", "1.0.0", CRATE);

    // Act
    let reply = api.publish(REPO, "foo-bar", "2.0.0", CRATE);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}
