//! dist-tags: добавление, список, удаление и ошибки.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::common::npm::TARBALL;
use crate::common::{Api, NPM_REPO};

fn dist_tags_uri(name: &str) -> String {
    format!(
        "/npm/{NPM_REPO}/-/package/{}/dist-tags",
        name.replace('/', "%2f")
    )
}

#[test]
fn dist_tags_can_be_added_listed_and_removed() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);
    let tag_uri = format!("{}/next", dist_tags_uri("@acme/left-pad"));
    let added = api.npm(Method::PUT, &tag_uri, br#""1.0.0""#.to_vec());
    let listed: Value = api.npm_get(&dist_tags_uri("@acme/left-pad")).json();

    // Act
    let removed = api.npm(Method::DELETE, &tag_uri, Vec::new());

    // Assert
    assert_eq!(added.status, StatusCode::OK);
    assert_eq!(listed, json!({"latest": "1.0.0", "next": "1.0.0"}));
    assert_eq!(removed.status, StatusCode::OK);
    assert_eq!(
        api.npm_get(&dist_tags_uri("@acme/left-pad")).json(),
        json!({"latest": "1.0.0"})
    );
}

#[test]
fn dist_tag_errors_are_reported_to_the_client() {
    // Arrange
    let api = Api::new();
    api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);
    let base = dist_tags_uri("@acme/left-pad");
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
