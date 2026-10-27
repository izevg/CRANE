//! Публикация через PUT: отказы и граничные формы документа.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::Api;
use crate::common::npm::{TARBALL, integrity, publish_doc};

use super::packument_uri;

#[test]
fn publish_with_wrong_sha512_is_rejected_and_nothing_is_published() {
    // Arrange
    let api = Api::new();
    let mut doc = publish_doc("left-pad", "1.0.0", TARBALL);
    doc["versions"]["1.0.0"]["dist"]["integrity"] = json!(integrity(b"other bytes"));

    // Act
    let reply = api.npm_put(&packument_uri("left-pad"), &doc);

    // Assert
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        api.npm_get(&packument_uri("left-pad")).status,
        StatusCode::NOT_FOUND
    );
}

#[test]
fn publishing_the_same_version_twice_conflicts() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Act
    let reply = api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Assert
    assert_eq!(reply.status, StatusCode::CONFLICT);
}

#[test]
fn malformed_publish_is_a_bad_request_with_the_reason() {
    // Arrange
    let api = Api::new();
    let mut doc = publish_doc("left-pad", "1.0.0", TARBALL);
    doc["versions"]["1.0.0"]["dependencies"] = json!({"evil": "github:a/b"});

    // Act
    let reply = api.npm_put(&packument_uri("left-pad"), &doc);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert!(reply.npm_error().contains("evil"), "{}", reply.text());
}

#[test]
fn put_without_attachments_or_versions_is_unsupported() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    let doc = json!({"name": "left-pad", "maintainers": [{"name": "x"}]});

    // Act
    let reply = api.npm_put(&packument_uri("left-pad"), &doc);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.npm_error(), "операция не поддерживается");
}

#[test]
fn oversized_publish_document_is_rejected_before_storage() {
    // Arrange
    let api = Api::new();
    let big = vec![7u8; crate::common::MAX_NPM_PUBLISH_BYTES];

    // Act
    let reply = api.npm_publish("left-pad", "1.0.0", &big);

    // Assert
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        api.npm_get(&packument_uri("left-pad")).status,
        StatusCode::NOT_FOUND
    );
}

#[test]
fn scoped_publish_with_the_full_name_attachment_key_npm_sends_is_accepted() {
    // Arrange
    let api = Api::new();
    let mut doc = publish_doc("@scope/pkg", "1.0.0", TARBALL);
    let attachment = doc["_attachments"]["pkg-1.0.0.tgz"].take();
    doc["_attachments"] = json!({"@scope/pkg-1.0.0.tgz": attachment});

    // Act
    let reply = api.npm_put("/npm/web/@scope%2fpkg", &doc);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        api.npm_get("/npm/web/@scope/pkg/-/pkg-1.0.0.tgz")
            .body
            .as_ref(),
        TARBALL
    );
}
