//! Заголовки кэша и учётных данных по классам ответов консоли.

use axum::http::Method;

use crate::common::TOKEN;
use crate::fixture::Console;

type Headers = (u16, Option<String>, Option<String>);
type Call<'a> = (Method, &'a str, &'a [(&'a str, &'a str)], &'a str);

fn cache_headers(
    console: &Console,
    method: Method,
    uri: &str,
    extra: &[(&str, &str)],
    body: &str,
) -> Headers {
    let reply = console.authed(method, uri, extra, body);
    let get = |name: &str| {
        reply
            .headers
            .get(name)
            .map(|v| v.to_str().unwrap().to_owned())
    };
    (reply.status.as_u16(), get("cache-control"), get("vary"))
}

fn stored(status: u16) -> Headers {
    (status, Some("no-store".to_owned()), None)
}

#[test]
fn settings_responses_are_never_stored() {
    // Arrange
    let console = Console::new();
    let create = [("if-none-match", "*")];
    let update = [("if-match", "\"1\"")];
    let remove = [("if-match", "\"2\"")];
    let body = r#"{"value": "file:///data"}"#;
    let calls: [Call; 6] = [
        (Method::GET, "/api/v1/settings", &[], ""),
        (Method::GET, "/api/v1/settings/blobs", &[], ""),
        (Method::PUT, "/api/v1/settings/blobs", &create, body),
        (Method::PUT, "/api/v1/settings/blobs", &update, body),
        (Method::DELETE, "/api/v1/settings/blobs", &remove, ""),
        (Method::GET, "/api/v1/settings/nope", &[], ""),
    ];

    // Act
    let actual =
        calls.map(|(method, uri, extra, body)| cache_headers(&console, method, uri, extra, body));

    // Assert
    assert_eq!(
        actual,
        [
            stored(200),
            stored(200),
            stored(200),
            stored(200),
            stored(200),
            stored(404),
        ]
    );
}

#[test]
fn profile_and_status_are_private_and_keyed_by_credentials() {
    // Arrange
    let console = Console::new();
    let tag = console.get("/api/v1/me").header("etag").to_owned();
    let calls: [(&str, &[(&str, &str)]); 3] = [
        ("/api/v1/me", &[]),
        ("/api/v1/status", &[]),
        ("/api/v1/me", &[("if-none-match", &tag)]),
    ];

    // Act
    let actual = calls.map(|(uri, extra)| cache_headers(&console, Method::GET, uri, extra, ""));

    // Assert
    let private = |status| {
        (
            status,
            Some("private".to_owned()),
            Some("Authorization, Cookie".to_owned()),
        )
    };
    assert_eq!(actual, [private(200), private(200), private(304)]);
}

#[test]
fn an_unauthenticated_settings_request_is_not_stored_either() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.call(Method::GET, "/api/v1/settings", &[], "");

    // Assert
    assert_eq!(
        (reply.status.as_u16(), reply.header("cache-control")),
        (401, "no-store")
    );
}

#[test]
fn a_cookie_never_replaces_or_rescues_the_bearer_credential() {
    // Arrange
    let console = Console::new();
    let cookie = ("cookie", "session=abc");
    let cases: [&[(&str, &str)]; 3] = [
        &[cookie],
        &[("authorization", "Bearer wrong"), cookie],
        &[("authorization", "Basic abc"), cookie],
    ];

    // Act
    let statuses = cases.map(|headers| {
        console
            .call(Method::GET, "/api/v1/me", headers, "")
            .status
            .as_u16()
    });

    // Assert
    assert_eq!(statuses, [401, 401, 401]);
}

#[test]
fn a_valid_bearer_wins_over_a_cookie_and_cross_origin_credentials_are_not_allowed() {
    // Arrange
    let console = Console::new();
    let bearer = format!("Bearer {TOKEN}");
    let headers = [
        ("authorization", bearer.as_str()),
        ("cookie", "session=abc"),
        ("origin", "https://evil.example"),
    ];

    // Act
    let reply = console.call(Method::GET, "/api/v1/me", &headers, "");

    // Assert
    let cors: Vec<_> = reply
        .headers
        .keys()
        .filter(|name| name.as_str().starts_with("access-control-"))
        .collect();
    assert_eq!((reply.status.as_u16(), cors.len()), (200, 0));
}
