//! Hosted-репозиторий npm через HTTP: публикация, чтение, устаревание, снятие, dist-tags.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::time::Duration;

use axum::http::{Method, StatusCode};
use common::npm::{TARBALL, file_of, integrity, publish_doc};
use common::{Api, NPM_REPO, PUBLIC_URL};
use serde_json::{Value, json};

const ABBREVIATED: &str = "application/vnd.npm.install-v1+json";

fn packument_uri(name: &str) -> String {
    format!("/npm/{NPM_REPO}/{}", name.replace('/', "%2f"))
}

fn tarball_uri(name: &str, version: &str) -> String {
    format!("/npm/{NPM_REPO}/{name}/-/{}", file_of(name, version))
}

fn delete_tarball_uri(name: &str, version: &str) -> String {
    format!("{}/-rev/1-x", tarball_uri(name, version))
}

fn dist_tags_uri(name: &str) -> String {
    format!(
        "/npm/{NPM_REPO}/-/package/{}/dist-tags",
        name.replace('/', "%2f")
    )
}

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
            ("authorization", &common::npm::bearer()),
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
            ("authorization", &common::npm::bearer()),
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
fn deprecation_through_put_shows_in_packument_and_can_be_cleared() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    let deprecate =
        json!({"name": "left-pad", "versions": {"1.0.0": {"deprecated": "use right-pad"}}});
    let put = api.npm_put(&packument_uri("left-pad"), &deprecate);
    let shown = api.npm_get(&packument_uri("left-pad")).json();
    let clear = json!({"name": "left-pad", "versions": {"1.0.0": {"deprecated": ""}}});

    // Act
    let cleared = api.npm_put(&packument_uri("left-pad"), &clear);

    // Assert
    assert_eq!(put.status, StatusCode::OK);
    assert_eq!(shown["versions"]["1.0.0"]["deprecated"], "use right-pad");
    assert_eq!(cleared.status, StatusCode::OK);
    let after = api.npm_get(&packument_uri("left-pad")).json();
    assert!(
        after["versions"]["1.0.0"].get("deprecated").is_none(),
        "{after}"
    );
}

#[test]
fn stale_document_that_omits_a_version_deletes_nothing() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    api.npm_publish("left-pad", "1.0.1", b"second");
    let stale = json!({"name": "left-pad", "versions": {"1.0.0": {"deprecated": "old"}}, "dist-tags": {"latest": "1.0.0"}});

    // Act
    let reply = api.npm_put(&packument_uri("left-pad"), &stale);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    let doc = api.npm_get(&packument_uri("left-pad")).json();
    let versions: Vec<&String> = doc["versions"].as_object().unwrap().keys().collect();
    assert_eq!(versions, ["1.0.0", "1.0.1"]);
    assert_eq!(doc["dist-tags"]["latest"], "1.0.1");
}

#[test]
fn deleting_a_tarball_inside_the_window_hides_the_version() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    api.npm_publish("left-pad", "1.0.1", b"second");
    api.advance(Duration::from_hours(1));

    // Act
    let reply = api.npm(
        Method::DELETE,
        &delete_tarball_uri("left-pad", "1.0.0"),
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    let doc = api.npm_get(&packument_uri("left-pad")).json();
    let versions: Vec<&String> = doc["versions"].as_object().unwrap().keys().collect();
    assert_eq!(versions, ["1.0.1"]);
    assert_eq!(
        api.npm_get(&tarball_uri("left-pad", "1.0.0")).status,
        StatusCode::GONE
    );
}

#[test]
fn deleted_version_number_cannot_be_published_again() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    api.npm(
        Method::DELETE,
        &delete_tarball_uri("left-pad", "1.0.0"),
        Vec::new(),
    );

    // Act
    let reply = api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Assert
    assert_eq!(reply.status, StatusCode::CONFLICT);
}

#[test]
fn deleting_a_tarball_outside_the_window_is_forbidden() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    api.advance(Duration::from_hours(73));

    // Act
    let reply = api.npm(
        Method::DELETE,
        &delete_tarball_uri("left-pad", "1.0.0"),
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(
        api.npm_get(&packument_uri("left-pad")).status,
        StatusCode::OK
    );
}

#[test]
fn deleting_a_package_removes_every_version() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    api.npm_publish("left-pad", "1.0.1", b"second");

    // Act
    let reply = api.npm(
        Method::DELETE,
        &format!("{}/-rev/1-x", packument_uri("left-pad")),
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        api.npm_get(&packument_uri("left-pad")).status,
        StatusCode::NOT_FOUND
    );
}

#[test]
fn dist_tags_can_be_added_listed_and_removed() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    let tag_uri = format!("{}/next", dist_tags_uri("left-pad"));
    let added = api.npm(Method::PUT, &tag_uri, br#""1.0.0""#.to_vec());
    let listed: Value = api.npm_get(&dist_tags_uri("left-pad")).json();

    // Act
    let removed = api.npm(Method::DELETE, &tag_uri, Vec::new());

    // Assert
    assert_eq!(added.status, StatusCode::OK);
    assert_eq!(listed, json!({"latest": "1.0.0", "next": "1.0.0"}));
    assert_eq!(removed.status, StatusCode::OK);
    assert_eq!(
        api.npm_get(&dist_tags_uri("left-pad")).json(),
        json!({"latest": "1.0.0"})
    );
}

#[test]
fn dist_tag_errors_are_reported_to_the_client() {
    // Arrange
    let api = Api::new();
    api.npm_publish("left-pad", "1.0.0", TARBALL);
    let base = dist_tags_uri("left-pad");
    let cases = [
        (Method::DELETE, format!("{base}/latest"), Vec::new()),
        (Method::DELETE, format!("{base}/missing"), Vec::new()),
        (Method::PUT, format!("{base}/1.0.0"), br#""1.0.0""#.to_vec()),
        (Method::PUT, format!("{base}/next"), br#""9.9.9""#.to_vec()),
        (Method::PUT, format!("{base}/next"), b"not json".to_vec()),
    ];

    // Act
    let statuses = cases.map(|(method, uri, body)| api.npm(method, &uri, body).status);

    // Assert
    assert_eq!(
        statuses,
        [
            StatusCode::BAD_REQUEST,
            StatusCode::NOT_FOUND,
            StatusCode::BAD_REQUEST,
            StatusCode::NOT_FOUND,
            StatusCode::BAD_REQUEST,
        ]
    );
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
    let reply = api.with_headers(Method::GET, &packument_uri("left-pad"), &[], Vec::new());

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.header("www-authenticate"), "Bearer realm=\"crane\"");
    assert_eq!(reply.npm_error(), "требуется токен");
}

#[test]
fn wrong_or_schemeless_token_is_forbidden_and_audited() {
    // Arrange
    let api = Api::new();
    let schemeless = common::TOKEN;
    let cases = ["Bearer wrong", schemeless, "Basic abc"];

    // Act
    let statuses = cases.map(|value| {
        api.with_headers(
            Method::GET,
            &packument_uri("left-pad"),
            &[("authorization", value)],
            Vec::new(),
        )
        .status
    });

    // Assert
    assert_eq!(statuses, [StatusCode::FORBIDDEN; 3]);
    assert_eq!(api.audit_events().len(), 3);
}

#[test]
fn oversized_publish_document_is_rejected_before_storage() {
    // Arrange
    let api = Api::new();
    let big = vec![7u8; common::MAX_NPM_PUBLISH_BYTES];

    // Act
    let reply = api.npm_publish("left-pad", "1.0.0", &big);

    // Assert
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        api.npm_get(&packument_uri("left-pad")).status,
        StatusCode::NOT_FOUND
    );
}
