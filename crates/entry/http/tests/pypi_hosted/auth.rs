//! Токен: без него 401 с Basic-challenge, с неверным 403, верный принимается и как Basic, и как Bearer.

use axum::http::{Method, StatusCode};

use crate::common::{Api, TOKEN};
use crate::fixtures::{Dist, simple_uri, upload_uri, yank_uri};

#[test]
fn request_without_a_token_gets_basic_challenge_on_every_route() {
    // Arrange
    let api = Api::new();
    let routes = [
        (Method::GET, simple_uri("acme-core")),
        (Method::POST, upload_uri()),
        (Method::PUT, yank_uri("acme-core", "1.0")),
    ];

    // Act
    let replies = routes.map(|(method, uri)| api.with_headers(method, &uri, &[], Vec::new()));

    // Assert
    let seen = replies.map(|reply| (reply.status, reply.header("www-authenticate").to_owned()));
    let expected = (StatusCode::UNAUTHORIZED, "Basic realm=\"crane\"".to_owned());
    assert_eq!(seen, [expected.clone(), expected.clone(), expected]);
}

#[test]
fn wrong_token_is_forbidden_and_publishes_nothing() {
    // Arrange
    let api = Api::new();
    let dist = Dist::wheel("acme-core", "1.0.0");

    // Act
    let reply = api.with_headers(
        Method::POST,
        &upload_uri(),
        &[
            ("authorization", "Basic X190b2tlbl9fOndyb25n"),
            ("content-type", &crate::fixtures::form_content_type()),
        ],
        dist.form(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(
        api.pypi_get(&simple_uri("acme-core"), None).status,
        StatusCode::NOT_FOUND
    );
}

#[test]
fn bearer_token_is_accepted_like_basic() {
    // Arrange
    let api = Api::new();
    api.pypi_published(&Dist::wheel("acme-core", "1.0.0"));
    let bearer = format!("Bearer {TOKEN}");

    // Act
    let reply = api.with_headers(
        Method::GET,
        &simple_uri("acme-core"),
        &[("authorization", &bearer)],
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}
