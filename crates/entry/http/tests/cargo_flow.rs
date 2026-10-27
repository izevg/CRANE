//! Полный cargo-сценарий через HTTP на in-memory адаптерах.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use axum::http::{Method, StatusCode};
use common::{Api, REPO, sha256_hex, single_index_line};
use serde_json::json;

const CRATE: &[u8] = b"\x1f\x8b fake crate bytes";

fn publish_ok() -> serde_json::Value {
    json!({ "warnings": { "invalid_categories": [], "invalid_badges": [], "other": [] } })
}

fn url(tail: &str) -> String {
    format!("/cargo/{REPO}/api/v1/crates/{tail}")
}

fn published() -> Api {
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.2.3", CRATE);
    api
}

fn yank(api: &Api) -> common::Reply {
    api.authed(Method::DELETE, &url("demo-crate/1.2.3/yank"), Vec::new())
}

fn unyank(api: &Api) -> common::Reply {
    api.authed(Method::PUT, &url("demo-crate/1.2.3/unyank"), Vec::new())
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
fn index_after_publish_describes_the_version_with_crate_checksum() {
    // Arrange
    let api = published();

    // Act
    let index = api.index_text("demo-crate");

    // Assert
    let line = single_index_line(&index);
    assert_eq!(line["name"], "demo-crate");
    assert_eq!(line["vers"], "1.2.3");
    assert_eq!(line["cksum"], sha256_hex(CRATE));
    assert_eq!(line["yanked"], false);
    assert_eq!(line["deps"][0]["name"], "serde");
    assert_eq!(line["features"]["default"], json!(["std"]));
}

#[test]
fn download_returns_published_bytes_with_content_length() {
    // Arrange
    let api = published();

    // Act
    let reply = api.get(&url("demo-crate/1.2.3/download"));

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), "application/octet-stream");
    assert_eq!(reply.header("content-length"), CRATE.len().to_string());
    assert_eq!(reply.body.as_ref(), CRATE);
}

#[test]
fn yank_answers_ok() {
    // Arrange
    let api = published();

    // Act
    let reply = yank(&api);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json(), json!({ "ok": true }));
}

#[test]
fn index_after_yank_marks_the_version_yanked() {
    // Arrange
    let api = published();
    yank(&api);

    // Act
    let index = api.index_text("demo-crate");

    // Assert
    assert_eq!(single_index_line(&index)["yanked"], true);
}

#[test]
fn yanked_version_is_still_downloadable() {
    // Arrange
    let api = published();
    yank(&api);

    // Act
    let reply = api.get(&url("demo-crate/1.2.3/download"));

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body.as_ref(), CRATE);
}

#[test]
fn unyank_answers_ok() {
    // Arrange
    let api = published();
    yank(&api);

    // Act
    let reply = unyank(&api);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json(), json!({ "ok": true }));
}

#[test]
fn index_after_unyank_marks_the_version_active_again() {
    // Arrange
    let api = published();
    yank(&api);
    unyank(&api);

    // Act
    let index = api.index_text("demo-crate");

    // Assert
    assert_eq!(single_index_line(&index)["yanked"], false);
}

#[test]
fn index_response_declares_plain_text_and_etag() {
    // Arrange
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Act
    let index = api.index_text("demo-crate");

    // Assert
    assert_eq!(index.header("content-type"), "text/plain; charset=utf-8");
    assert_eq!(
        index.header("etag"),
        format!("\"{}\"", sha256_hex(&index.body))
    );
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
    let mut body = common::publish_body("demo-crate", "1.0.0", CRATE);
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
    let big = vec![0_u8; common::MAX_PUBLISH_BYTES + 1];

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
    let overhead = common::publish_body("demo-crate", "1.0.0", &[]).len();
    let fits = vec![0_u8; common::MAX_PUBLISH_BYTES - overhead];

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

#[test]
fn index_of_unknown_crate_is_not_found() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.index_text("ghost-crate");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.header("content-type"), "application/json");
}

#[test]
fn index_with_non_canonical_path_is_not_found() {
    // Arrange
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Act
    let reply = api.get(&format!("/cargo/{REPO}/index/xx/yy/demo-crate"));

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[test]
fn index_of_unknown_repository_is_not_found() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get("/cargo/nope/index/de/mo/demo-crate");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[test]
fn download_of_unknown_version_is_not_found() {
    // Arrange
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Act
    let reply = api.get(&url("demo-crate/9.9.9/download"));

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[test]
fn download_with_malformed_version_is_bad_request() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get(&url("demo-crate/not-a-version/download"));

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.header("content-type"), "application/json");
}

#[test]
fn yank_of_unknown_version_is_not_found() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.authed(Method::DELETE, &url("demo-crate/1.0.0/yank"), Vec::new());

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[test]
fn owners_of_published_crate_is_the_authenticated_principal() {
    // Arrange
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Act
    let reply = api.get(&url("demo-crate/owners"));

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.json(),
        json!({ "users": [{ "id": 1, "login": "crane", "name": null }] })
    );
}

#[test]
fn owners_of_unknown_crate_is_not_found() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get(&url("ghost-crate/owners"));

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[test]
fn owners_put_and_delete_are_not_implemented() {
    // Arrange
    let api = Api::new();

    // Act
    let replies = [Method::PUT, Method::DELETE]
        .map(|method| api.authed(method, &url("demo-crate/owners"), Vec::new()))
        .map(|reply| (reply.status, reply.detail()));

    // Assert
    let expected = (
        StatusCode::NOT_IMPLEMENTED,
        "управление владельцами появится в v0.4.0".to_owned(),
    );
    assert_eq!(replies, [expected.clone(), expected]);
}

#[test]
fn unknown_path_outside_cargo_is_problem_json() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get("/nope");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.header("content-type"), "application/problem+json");
    let body = reply.json();
    assert_eq!(
        (&body["status"], &body["type"], &body["title"]),
        (&json!(404), &json!("about:blank"), &json!("Not Found"))
    );
}

#[test]
fn unknown_path_inside_cargo_is_cargo_shaped_not_found() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get(&format!("/cargo/{REPO}/whatever"));

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.header("content-type"), "application/json");
}
