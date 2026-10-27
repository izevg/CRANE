//! Чтение: индекс и скачивание, включая ненайденное.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::cargo::{sha256_hex, single_index_line};
use crate::common::{Api, REPO};

use super::{CRATE, published, url};

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
