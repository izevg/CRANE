#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use axum::http::HeaderValue;
use serde_json::json;

use super::*;

fn request(if_none_match: &[&str]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for value in if_none_match {
        headers.append(header::IF_NONE_MATCH, HeaderValue::from_str(value).unwrap());
    }
    headers
}

fn etag(response: &Response) -> String {
    response.headers()[header::ETAG]
        .to_str()
        .unwrap()
        .to_owned()
}

#[test]
fn etag_is_the_same_for_equal_bodies_and_differs_for_different_ones() {
    // Arrange
    let bodies = [
        json!({"a": 1, "b": 2}),
        json!({"b": 2, "a": 1}),
        json!({"a": 2}),
    ];

    // Act
    let tags = bodies.map(|body| etag(&cacheable(&request(&[]), &body, "private")));

    // Assert
    assert_eq!((tags[0] == tags[1], tags[0] == tags[2]), (true, false));
}

#[test]
fn if_none_match_with_the_current_tag_gives_304_without_body() {
    // Arrange
    let body = json!({"a": 1});
    let tag = etag(&cacheable(&request(&[]), &body, "private"));
    let wildcards = [
        tag.clone(),
        format!("W/{tag}"),
        "*".to_owned(),
        format!("\"x\", {tag}"),
    ];

    // Act
    let statuses = wildcards.map(|value| cacheable(&request(&[&value]), &body, "private").status());

    // Assert
    assert_eq!(statuses, [StatusCode::NOT_MODIFIED; 4]);
}

#[test]
fn if_none_match_with_a_stale_tag_gives_the_body() {
    // Arrange
    let body = json!({"a": 1});

    // Act
    let response = cacheable(&request(&["\"stale\""]), &body, "private");

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn not_modified_still_carries_the_tag_and_cache_policy() {
    // Arrange
    let body = json!({"a": 1});
    let tag = etag(&cacheable(&request(&[]), &body, "private, max-age=30"));

    // Act
    let response = cacheable(&request(&[&tag]), &body, "private, max-age=30");

    // Assert
    assert_eq!(
        (
            etag(&response),
            response.headers()[header::CACHE_CONTROL].to_str().unwrap()
        ),
        (tag, "private, max-age=30")
    );
}

#[test]
fn json_exposes_the_row_version_as_etag_and_nothing_without_a_row() {
    // Arrange
    let body = json!({});
    let versions = [Version::new(7), None];

    // Act
    let tags = versions.map(|version| {
        json(StatusCode::OK, &body, version)
            .headers()
            .get(header::ETAG)
            .cloned()
    });

    // Assert
    assert_eq!(tags, [Some(HeaderValue::from_static("\"7\"")), None]);
}
