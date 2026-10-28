//! Yank и unyank в формате cargo.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::Api;
use crate::common::cargo::single_index_line;

use super::{CRATE, published, url};

fn yank(api: &Api) -> crate::common::Reply {
    api.authed(Method::DELETE, &url("demo-crate/1.2.3/yank"), Vec::new())
}

fn unyank(api: &Api) -> crate::common::Reply {
    api.authed(Method::PUT, &url("demo-crate/1.2.3/unyank"), Vec::new())
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
fn yank_of_unknown_version_is_not_found() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.authed(Method::DELETE, &url("demo-crate/1.0.0/yank"), Vec::new());

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}
