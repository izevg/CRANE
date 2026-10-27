//! Чтение hosted npm: packument, tarball, HEAD, условные запросы, scoped-имена.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::npm::{TARBALL, integrity};
use crate::common::{Api, NPM_REPO, PUBLIC_URL};

use super::{packument_uri, tarball_uri};

const ABBREVIATED: &str = "application/vnd.npm.install-v1+json";

#[test]
fn published_version_appears_in_full_packument_with_our_tarball_url_and_integrity() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Act
    let reply = api.npm_get(&packument_uri("left-pad"));

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(reply.header("vary"), "Accept");
    let doc = reply.json();
    let version = &doc["versions"]["1.0.0"];
    assert_eq!(doc["dist-tags"], json!({"latest": "1.0.0"}));
    assert_eq!(
        version["dist"]["tarball"],
        format!("{PUBLIC_URL}/npm/{NPM_REPO}/left-pad/-/left-pad-1.0.0.tgz")
    );
    assert_eq!(version["dist"]["integrity"], integrity(TARBALL));
    assert_eq!(version["_id"], "left-pad@1.0.0");
}

#[test]
fn abbreviated_packument_is_served_only_when_requested() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Act
    let reply = api.with_headers(
        Method::GET,
        &packument_uri("left-pad"),
        &[
            ("authorization", &crate::common::npm::bearer()),
            ("accept", ABBREVIATED),
        ],
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.header("content-type"), ABBREVIATED);
    assert_eq!(reply.header("vary"), "Accept");
    let doc = reply.json();
    assert_eq!(doc["name"], "left-pad");
    assert!(doc.get("_id").is_none(), "{doc}");
    assert_eq!(
        doc["versions"]["1.0.0"]["dependencies"],
        json!({"left": "^1.0.0"})
    );
}

#[test]
fn packument_supports_conditional_requests() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    let etag = api
        .npm_get(&packument_uri("left-pad"))
        .header("etag")
        .to_owned();

    // Act
    let reply = api.with_headers(
        Method::GET,
        &packument_uri("left-pad"),
        &[
            ("authorization", &crate::common::npm::bearer()),
            ("if-none-match", &etag),
        ],
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_MODIFIED);
}

#[test]
fn tarball_is_served_byte_for_byte() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Act
    let reply = api.npm_get(&tarball_uri("left-pad", "1.0.0"));

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body.as_ref(), TARBALL);
    assert_eq!(reply.header("content-length"), TARBALL.len().to_string());
}

#[test]
fn head_of_packument_is_answered_like_get() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Act
    let reply = api.npm(Method::HEAD, &packument_uri("left-pad"), Vec::new());

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}

#[test]
fn unknown_package_is_not_found_in_npm_shape() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.npm_get(&packument_uri("nothing"));

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.npm_error(), "не найдено");
}

#[test]
fn scoped_package_is_reachable_by_escaped_and_plain_slash() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@scope/pkg", "1.0.0", TARBALL);
    let uris = [
        "/npm/web/@scope%2fpkg".to_owned(),
        "/npm/web/%40scope%2fpkg".to_owned(),
        "/npm/web/@scope/pkg".to_owned(),
    ];

    // Act
    let replies = uris.each_ref().map(|uri| api.npm_get(uri));

    // Assert
    assert_eq!(
        replies.each_ref().map(|reply| reply.status),
        [StatusCode::OK; 3]
    );
    assert_eq!(
        replies[0].json()["versions"]["1.0.0"]["dist"]["tarball"],
        format!("{PUBLIC_URL}/npm/{NPM_REPO}/@scope/pkg/-/pkg-1.0.0.tgz")
    );
    assert_eq!(
        api.npm_get("/npm/web/@scope/pkg/-/pkg-1.0.0.tgz")
            .body
            .as_ref(),
        TARBALL
    );
}

#[test]
fn bad_package_name_is_a_bad_request() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.npm_get("/npm/web/.hidden");

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
}
