//! Yank и unyank (PEP 592) временным маршрутом формата.

use axum::http::{Method, StatusCode};

use crate::common::Api;
use crate::fixtures::{Dist, JSON_TYPE, file_uri, simple_uri, yank_uri};

const WHEEL: &str = "acme_core-1.0.0-py3-none-any.whl";

fn two_versions() -> Api {
    let api = Api::new();
    api.pypi_published(&Dist::wheel("acme-core", "1.0.0"));
    api.pypi_published(&Dist::sdist("acme-core", "1.0.0"));
    api.pypi_published(&Dist::wheel("acme-core", "2.0"));
    api
}

#[test]
fn yanked_version_is_marked_in_html_and_json_for_all_its_files_only() {
    // Arrange
    let api = two_versions();

    // Act
    let reply = api.pypi_yank(Method::PUT, "acme-core", "1.0.0");

    // Assert
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let html = api.pypi_get(&simple_uri("acme-core"), None).anchors();
    let marked: Vec<bool> = html.iter().map(|a| a.contains(" data-yanked>")).collect();
    let json = api
        .pypi_get(&simple_uri("acme-core"), Some(JSON_TYPE))
        .json();
    let flags: Vec<bool> = json["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["yanked"].as_bool().unwrap())
        .collect();
    assert_eq!(
        (marked, flags),
        (vec![true, true, false], vec![true, true, false])
    );
}

#[test]
fn unyank_removes_the_mark() {
    // Arrange
    let api = two_versions();
    api.pypi_yank(Method::PUT, "acme-core", "1.0.0");

    // Act
    let reply = api.pypi_yank(Method::DELETE, "acme-core", "1.0.0");

    // Assert
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    let html = api.pypi_get(&simple_uri("acme-core"), None).text();
    assert!(!html.contains("data-yanked"), "{html}");
}

#[test]
fn yank_and_unyank_are_idempotent() {
    // Arrange
    let api = two_versions();

    // Act
    let statuses = [
        api.pypi_yank(Method::PUT, "acme-core", "1.0.0").status,
        api.pypi_yank(Method::PUT, "acme-core", "1.0.0").status,
        api.pypi_yank(Method::DELETE, "acme-core", "2.0").status,
    ];

    // Assert
    assert_eq!(statuses, [StatusCode::NO_CONTENT; 3]);
}

#[test]
fn yanked_file_stays_downloadable_for_lock_files() {
    // Arrange
    let api = two_versions();
    api.pypi_yank(Method::PUT, "acme-core", "1.0.0");

    // Act
    let reply = api.pypi_get(&file_uri("acme-core", "1", WHEEL), None);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}

#[test]
fn yank_accepts_any_spelling_of_name_and_version() {
    // Arrange
    let api = two_versions();

    // Act
    let reply = api.pypi_yank(Method::PUT, "Acme_Core", "1");

    // Assert
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
}

#[test]
fn yank_of_unknown_version_is_not_found_and_of_malformed_one_is_a_bad_request() {
    // Arrange
    let api = two_versions();

    // Act
    let statuses = [
        api.pypi_yank(Method::PUT, "acme-core", "9.9").status,
        api.pypi_yank(Method::PUT, "acme-core", "latest").status,
        api.pypi_yank(Method::PUT, "-bad-", "1.0").status,
    ];

    // Assert
    assert_eq!(
        statuses,
        [
            StatusCode::NOT_FOUND,
            StatusCode::BAD_REQUEST,
            StatusCode::BAD_REQUEST
        ]
    );
}

#[test]
fn yank_with_a_wrong_token_is_forbidden_and_changes_nothing() {
    // Arrange
    let api = two_versions();

    // Act
    let reply = api.with_headers(
        Method::PUT,
        &yank_uri("acme-core", "1.0.0"),
        &[("authorization", "Basic X190b2tlbl9fOndyb25n")],
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let html = api.pypi_get(&simple_uri("acme-core"), None).text();
    assert!(!html.contains("data-yanked"), "{html}");
}
