//! Владельцы crate.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{Api, REPO};

use super::{CRATE, url};

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
