//! Чтение: страница проекта по Accept, редиректы на нормализованный адрес, файлы.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{Api, PUBLIC_URL, PYPI_REPO};
use crate::fixtures::{
    Dist, JSON_TYPE, SDIST_BYTES, WHEEL_BYTES, file_uri, sha256_hex, simple_uri,
};

const WHEEL: &str = "acme_core-1.0.0-py3-none-any.whl";

fn published() -> Api {
    let api = Api::new();
    api.pypi_published(&Dist::wheel("acme-core", "1.0.0"));
    api
}

#[test]
fn json_page_is_served_when_the_client_names_it_with_vary_and_exact_content_type() {
    // Arrange
    let api = published();

    // Act
    let reply = api.pypi_get(&simple_uri("acme-core"), Some(JSON_TYPE));

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), JSON_TYPE);
    assert_eq!(reply.header("vary"), "Accept");
    let mut page = reply.json();
    let file = page["files"][0].as_object_mut().unwrap();
    assert!(
        file.remove("upload-time")
            .unwrap()
            .as_str()
            .unwrap()
            .ends_with('Z')
    );
    assert_eq!(
        page,
        json!({
            "meta": {"api-version": "1.1"},
            "name": "acme-core",
            "versions": ["1"],
            "files": [{
                "filename": WHEEL,
                "url": format!("{PUBLIC_URL}/pypi/{PYPI_REPO}/files/acme-core/1/{WHEEL}"),
                "hashes": {"sha256": sha256_hex(WHEEL_BYTES)},
                "requires-python": ">=3.8",
                "yanked": false,
                "size": WHEEL_BYTES.len(),
            }],
        })
    );
}

#[test]
fn html_page_is_the_default_and_carries_hash_and_requires_python() {
    // Arrange
    let api = published();

    // Act
    let replies = [
        api.pypi_get(&simple_uri("acme-core"), None),
        api.pypi_get(&simple_uri("acme-core"), Some("*/*")),
        api.pypi_get(&simple_uri("acme-core"), Some("text/html")),
    ];

    // Assert
    let seen = replies.map(|reply| {
        (
            reply.header("content-type").to_owned(),
            reply.header("vary").to_owned(),
            reply.anchors(),
        )
    });
    let anchor = format!(
        "<a href=\"{PUBLIC_URL}/pypi/{PYPI_REPO}/files/acme-core/1/{WHEEL}#sha256={}\" data-requires-python=\"&gt;=3.8\">{WHEEL}</a><br/>",
        sha256_hex(WHEEL_BYTES)
    );
    let expected = (
        "text/html; charset=utf-8".to_owned(),
        "Accept".to_owned(),
        vec![anchor],
    );
    assert_eq!(seen, [expected.clone(), expected.clone(), expected]);
}

#[test]
fn page_supports_conditional_requests() {
    // Arrange
    let api = published();
    let etag = api
        .pypi_get(&simple_uri("acme-core"), None)
        .header("etag")
        .to_owned();

    // Act
    let reply = api.pypi(
        Method::GET,
        &simple_uri("acme-core"),
        &[("if-none-match", &etag)],
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_MODIFIED);
}

#[test]
fn unnormalized_name_and_missing_slash_are_permanent_redirects() {
    // Arrange
    let api = published();
    let uris = [
        "/pypi/py/simple/Acme_Core/",
        "/pypi/py/simple/acme-core",
        "/pypi/py/simple/Acme.Core",
    ];

    // Act
    let replies = uris.map(|uri| api.pypi_get(uri, None));

    // Assert
    let seen = replies.map(|reply| (reply.status, reply.header("location").to_owned()));
    assert_eq!(
        seen,
        [
            (StatusCode::MOVED_PERMANENTLY, "../acme-core/".to_owned()),
            (StatusCode::MOVED_PERMANENTLY, "acme-core/".to_owned()),
            (StatusCode::MOVED_PERMANENTLY, "acme-core/".to_owned()),
        ]
    );
}

#[test]
fn unknown_project_invalid_name_and_the_root_index_are_not_found() {
    // Arrange
    let api = published();
    let uris = [
        "/pypi/py/simple/missing/",
        "/pypi/py/simple/-bad-/",
        "/pypi/py/simple/",
        "/pypi/py/simple",
    ];

    // Act
    let statuses = uris.map(|uri| api.pypi_get(uri, None).status);

    // Assert
    assert_eq!(statuses, [StatusCode::NOT_FOUND; 4]);
}

#[test]
fn file_is_downloaded_byte_for_byte() {
    // Arrange
    let api = published();

    // Act
    let reply = api.pypi_get(&file_uri("acme-core", "1", WHEEL), None);

    // Assert
    assert_eq!(sha256_hex(&reply.body), sha256_hex(WHEEL_BYTES));
}

#[test]
fn head_of_a_file_answers_with_its_length_and_without_a_body() {
    // Arrange
    let api = published();
    let uri = file_uri("acme-core", "1", WHEEL);

    // Act
    let head = api.pypi(Method::HEAD, &uri, &[], Vec::new());

    // Assert
    assert_eq!(
        (
            head.status,
            head.header("content-length").to_owned(),
            head.body.len()
        ),
        (StatusCode::OK, WHEEL_BYTES.len().to_string(), 0)
    );
}

#[test]
fn file_is_found_under_any_spelling_of_the_version() {
    // Arrange
    let api = published();

    // Act
    let reply = api.pypi_get(&file_uri("Acme_Core", "1.0.0", WHEEL), None);

    // Assert
    assert_eq!(reply.body.as_ref(), WHEEL_BYTES);
}

#[test]
fn unknown_file_version_or_name_is_not_found() {
    // Arrange
    let api = published();
    let uris = [
        file_uri("acme-core", "1", "other.whl"),
        file_uri("acme-core", "2", WHEEL),
        file_uri("missing", "1", WHEEL),
        file_uri("acme-core", "not-a-version", WHEEL),
    ];

    // Act
    let statuses = uris.map(|uri| api.pypi_get(&uri, None).status);

    // Assert
    assert_eq!(statuses, [StatusCode::NOT_FOUND; 4]);
}

#[test]
fn sdist_is_downloadable_with_its_own_bytes() {
    // Arrange
    let api = Api::new();
    api.pypi_published(&Dist::sdist("acme-core", "1.0.0"));

    // Act
    let reply = api.pypi_get(&file_uri("acme-core", "1", "acme_core-1.0.0.tar.gz"), None);

    // Assert
    assert_eq!(reply.body.as_ref(), SDIST_BYTES);
}
