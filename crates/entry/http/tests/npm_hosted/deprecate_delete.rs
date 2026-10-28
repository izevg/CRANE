//! Устаревание и снятие: deprecate через PUT, окно удаления, надгробия.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::Api;
use crate::common::npm::TARBALL;

use super::{packument_uri, tarball_uri};

fn delete_tarball_uri(name: &str, version: &str) -> String {
    format!("{}/-rev/1-x", tarball_uri(name, version))
}

#[test]
fn deprecation_through_put_shows_in_packument_and_can_be_cleared() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);
    let deprecate =
        json!({"name": "@acme/left-pad", "versions": {"1.0.0": {"deprecated": "use right-pad"}}});
    let put = api.npm_put(&packument_uri("@acme/left-pad"), &deprecate);
    let shown = api.npm_get(&packument_uri("@acme/left-pad")).json();
    let clear = json!({"name": "@acme/left-pad", "versions": {"1.0.0": {"deprecated": ""}}});

    // Act
    let cleared = api.npm_put(&packument_uri("@acme/left-pad"), &clear);

    // Assert
    assert_eq!(put.status, StatusCode::OK);
    assert_eq!(shown["versions"]["1.0.0"]["deprecated"], "use right-pad");
    assert_eq!(cleared.status, StatusCode::OK);
    let after = api.npm_get(&packument_uri("@acme/left-pad")).json();
    assert!(
        after["versions"]["1.0.0"].get("deprecated").is_none(),
        "{after}"
    );
}

#[test]
fn stale_document_that_omits_a_version_deletes_nothing() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);
    api.npm_publish("@acme/left-pad", "1.0.1", b"second");
    let stale = json!({"name": "@acme/left-pad", "versions": {"1.0.0": {"deprecated": "old"}}, "dist-tags": {"latest": "1.0.0"}});

    // Act
    let reply = api.npm_put(&packument_uri("@acme/left-pad"), &stale);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    let doc = api.npm_get(&packument_uri("@acme/left-pad")).json();
    let versions: Vec<&String> = doc["versions"].as_object().unwrap().keys().collect();
    assert_eq!(versions, ["1.0.0", "1.0.1"]);
    assert_eq!(doc["dist-tags"]["latest"], "1.0.1");
}

#[test]
fn deleting_a_tarball_inside_the_window_hides_the_version() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);
    api.npm_publish("@acme/left-pad", "1.0.1", b"second");
    api.advance(Duration::from_hours(1));

    // Act
    let reply = api.npm(
        Method::DELETE,
        &delete_tarball_uri("@acme/left-pad", "1.0.0"),
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    let doc = api.npm_get(&packument_uri("@acme/left-pad")).json();
    let versions: Vec<&String> = doc["versions"].as_object().unwrap().keys().collect();
    assert_eq!(versions, ["1.0.1"]);
    assert_eq!(
        api.npm_get(&tarball_uri("@acme/left-pad", "1.0.0")).status,
        StatusCode::GONE
    );
}

#[test]
fn deleted_version_number_cannot_be_published_again() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);
    api.npm(
        Method::DELETE,
        &delete_tarball_uri("@acme/left-pad", "1.0.0"),
        Vec::new(),
    );

    // Act
    let reply = api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);

    // Assert
    assert_eq!(reply.status, StatusCode::CONFLICT);
}

#[test]
fn deleting_a_tarball_outside_the_window_is_forbidden() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);
    api.advance(Duration::from_hours(73));

    // Act
    let reply = api.npm(
        Method::DELETE,
        &delete_tarball_uri("@acme/left-pad", "1.0.0"),
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(
        api.npm_get(&packument_uri("@acme/left-pad")).status,
        StatusCode::OK
    );
}

#[test]
fn deleting_a_package_removes_every_version() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);
    api.npm_publish("@acme/left-pad", "1.0.1", b"second");

    // Act
    let reply = api.npm(
        Method::DELETE,
        &format!("{}/-rev/1-x", packument_uri("@acme/left-pad")),
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        api.npm_get(&packument_uri("@acme/left-pad")).status,
        StatusCode::NOT_FOUND
    );
}
