//! Аутентификация, `config.json`, условные запросы индекса и admin-пробы.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use axum::http::{Method, StatusCode};
use common::{Api, PUBLIC_URL, REPO, TOKEN};
use serde_json::json;

const CONFIG_URL: &str = "/cargo/internal/index/config.json";
const CRATE: &[u8] = b"bytes";

fn config_with(headers: &[(&str, &str)]) -> common::Reply {
    Api::new().with_headers(Method::GET, CONFIG_URL, headers, Vec::new())
}

#[test]
fn config_without_authorization_is_unauthorized_and_asks_for_cargo_token() {
    // Arrange
    let headers = [];

    // Act
    let reply = config_with(&headers);

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.header("www-authenticate"), "Cargo");
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(reply.detail(), "требуется токен");
}

#[test]
fn config_with_wrong_token_is_forbidden() {
    // Arrange
    let headers = [("authorization", "wrong")];

    // Act
    let reply = config_with(&headers);

    // Assert
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.detail(), "токен отвергнут");
}

#[test]
fn bearer_scheme_is_not_stripped() {
    // Arrange
    let presented = format!("Bearer {TOKEN}");

    // Act
    let reply = config_with(&[("authorization", presented.as_str())]);

    // Assert
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}

#[test]
fn config_with_right_token_points_clients_at_this_repository() {
    // Arrange
    let headers = [("authorization", TOKEN)];

    // Act
    let reply = config_with(&headers);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(
        reply.json(),
        json!({
            "dl": format!("{PUBLIC_URL}/cargo/{REPO}/api/v1/crates"),
            "api": format!("{PUBLIC_URL}/cargo/{REPO}"),
            "auth-required": true
        })
    );
}

#[test]
fn unknown_cargo_path_without_token_is_unauthorized() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.with_headers(Method::GET, "/cargo/x/whatever", &[], Vec::new());

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}

#[test]
fn publish_without_token_is_unauthorized() {
    // Arrange
    let api = Api::new();
    let body = common::cargo::publish_body("demo-crate", "1.0.0", CRATE);

    // Act
    let reply = api.with_headers(Method::PUT, "/cargo/internal/api/v1/crates/new", &[], body);

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}

#[test]
fn admin_probes_need_no_token() {
    // Arrange
    let api = Api::new();
    let paths = ["/healthz/live", "/healthz/ready"];

    // Act
    let replies = paths.map(|path| {
        let reply = api.admin_get(path);
        (reply.status, reply.text())
    });

    // Assert
    let ok = (StatusCode::OK, "ok".to_owned());
    assert_eq!(replies, [ok.clone(), ok]);
}

#[test]
fn admin_unknown_path_is_problem_json() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.admin_get("/metrics");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.header("content-type"), "application/problem+json");
}

/// Опубликованный крейт и `ETag` его индекса.
fn published_with_etag() -> (Api, String) {
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.0.0", CRATE);
    let etag = api.index_text("demo-crate").header("etag").to_owned();
    (api, etag)
}

fn index_if_none_match(api: &Api, condition: &str) -> common::Reply {
    api.with_headers(
        Method::GET,
        &format!(
            "/cargo/{REPO}/index/{}",
            common::cargo::index_path("demo-crate")
        ),
        &[("authorization", TOKEN), ("if-none-match", condition)],
        Vec::new(),
    )
}

#[test]
fn matching_if_none_match_gives_not_modified_with_etag_and_no_body() {
    // Arrange
    let (api, etag) = published_with_etag();

    // Act
    let reply = index_if_none_match(&api, &etag);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_MODIFIED);
    assert_eq!(reply.header("etag"), etag);
    assert!(reply.body.is_empty());
}

#[test]
fn different_if_none_match_gives_full_response() {
    // Arrange
    let (api, etag) = published_with_etag();

    // Act
    let reply = index_if_none_match(&api, "\"other\"");

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("etag"), etag);
    assert!(!reply.body.is_empty());
}

#[test]
fn weak_form_of_same_etag_gives_not_modified() {
    // Arrange
    let (api, etag) = published_with_etag();
    let weak = format!("W/{etag}");

    // Act
    let reply = index_if_none_match(&api, &weak);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_MODIFIED);
}

#[test]
fn wildcard_if_none_match_gives_not_modified() {
    // Arrange
    let (api, _) = published_with_etag();

    // Act
    let reply = index_if_none_match(&api, "*");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_MODIFIED);
}

#[test]
fn etag_changes_when_the_index_changes() {
    // Arrange
    let (api, before) = published_with_etag();
    api.publish(REPO, "demo-crate", "1.1.0", CRATE);

    // Act
    let after = api.index_text("demo-crate");

    // Assert
    assert_ne!(after.header("etag"), before);
}

#[test]
fn config_of_unknown_repository_is_cargo_shaped_not_found() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get("/cargo/nope/index/config.json");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(reply.detail(), "репозиторий не найден");
}

#[test]
fn config_of_known_repository_points_to_its_own_base() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get(CONFIG_URL);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.json()["dl"],
        format!("{PUBLIC_URL}/cargo/internal/api/v1/crates")
    );
}
