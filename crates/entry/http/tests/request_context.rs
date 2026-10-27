//! Контекст запроса: `X-Request-Id`, адрес клиента и тела ошибок с идентификатором.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use axum::http::{Method, StatusCode};
use common::{Api, REPO, TOKEN};
use uuid::Uuid;

const CONFIG_URL: &str = "/cargo/internal/index/config.json";
const TRUSTED_PEER: &str = "127.0.0.1:40000";
const STRANGER_PEER: &str = "203.0.113.5:40000";

fn auth() -> [(&'static str, &'static str); 1] {
    [("authorization", TOKEN)]
}

/// Заголовок — UUID версии 7, то есть сгенерирован сервером.
fn is_generated(id: &str) -> bool {
    Uuid::parse_str(id).is_ok_and(|uuid| uuid.get_version_num() == 7)
}

#[test]
fn trusted_peer_request_id_is_echoed() {
    // Arrange
    let api = Api::with_proxies("127.0.0.1");

    // Act
    let reply = api.get_from(
        TRUSTED_PEER,
        CONFIG_URL,
        &[("authorization", TOKEN), ("x-request-id", "abc-1")],
    );

    // Assert
    assert_eq!(reply.header("x-request-id"), "abc-1");
}

#[test]
fn untrusted_peer_request_id_is_replaced_by_generated_v7() {
    // Arrange
    let api = Api::with_proxies("127.0.0.1");

    // Act
    let reply = api.get_from(
        STRANGER_PEER,
        CONFIG_URL,
        &[("authorization", TOKEN), ("x-request-id", "abc-1")],
    );

    // Assert
    let id = reply.header("x-request-id");
    assert_ne!(id, "abc-1");
    assert!(is_generated(id), "{id}");
}

#[test]
fn request_id_without_known_peer_is_never_taken_from_the_client() {
    // Arrange
    let api = Api::with_proxies("127.0.0.1");
    let headers = [("authorization", TOKEN), ("x-request-id", "abc-1")];

    // Act
    let reply = api.with_headers(Method::GET, CONFIG_URL, &headers, Vec::new());

    // Assert
    assert!(is_generated(reply.header("x-request-id")));
}

#[test]
fn invalid_request_id_from_trusted_peer_is_replaced() {
    // Arrange
    let api = Api::with_proxies("127.0.0.1");

    // Act
    let reply = api.get_from(
        TRUSTED_PEER,
        CONFIG_URL,
        &[("authorization", TOKEN), ("x-request-id", "has space")],
    );

    // Assert
    assert!(is_generated(reply.header("x-request-id")));
}

#[test]
fn cargo_error_detail_ends_with_the_request_id_from_the_header() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get(&format!("/cargo/{REPO}/whatever"));

    // Assert
    let detail = reply.json()["errors"][0]["detail"]
        .as_str()
        .unwrap()
        .to_owned();
    let id = reply.header("x-request-id");
    assert_eq!(detail, format!("не найдено (request_id {id})"));
}

#[test]
fn problem_json_carries_the_request_id_from_the_header() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get("/nope");

    // Assert
    let body = reply.json();
    assert_eq!(body["request_id"], reply.header("x-request-id"));
    assert_eq!(body["detail"], "маршрут не найден");
}

#[test]
fn admin_problem_json_carries_the_request_id_from_the_header() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.admin_get("/metrics");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.json()["request_id"], reply.header("x-request-id"));
}

#[test]
fn unauthorized_reply_keeps_www_authenticate_and_gets_the_suffix() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get_from(STRANGER_PEER, CONFIG_URL, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.header("www-authenticate"), "Cargo");
    assert_eq!(reply.detail(), "требуется токен");
}

#[test]
fn successful_reply_still_carries_the_request_id_header() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get_from(STRANGER_PEER, CONFIG_URL, &auth());

    // Assert
    assert!(is_generated(reply.header("x-request-id")));
}
