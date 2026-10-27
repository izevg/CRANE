//! Неизвестные пути внутри и вне cargo.

use axum::http::StatusCode;
use serde_json::json;

use crate::common::{Api, REPO};

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
