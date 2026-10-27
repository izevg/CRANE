//! Proxy-репозиторий npm через HTTP: фейковый upstream, memory-адаптеры, сети нет.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::http::{Method, StatusCode};
use common::npm::{TARBALL, bearer, integrity};
use common::proxy::{FakeUpstream, NPM_NAME, NpmProxy, Step};
use common::{Api, PUBLIC_URL, Reply, capture_logs};
use crane_memory::ManualClock;
use serde_json::{Value, json};

const URI: &str = "/npm/npmjs/left-pad";
const TARBALL_URI: &str = "/npm/npmjs/left-pad/-/left-pad-1.0.0.tgz";
const TARBALL_PATH: &str = "left-pad/-/left-pad-1.0.0.tgz";
const ABBREVIATED: &str = "application/vnd.npm.install-v1+json";
const EXPIRED_AFTER: Duration = Duration::from_secs(700);

struct World {
    api: Api,
    upstream: FakeUpstream,
    clock: Arc<ManualClock>,
}

fn world() -> World {
    let proxy = NpmProxy::new();
    World {
        api: Api::with_npm_proxy_repos(vec![(NPM_NAME.to_owned(), proxy.router)]),
        upstream: proxy.upstream,
        clock: proxy.clock,
    }
}

/// Packument upstream: ссылка ведёт на чужой хост, который в ответе появляться не должен.
fn packument(integrity_field: &Value) -> Vec<u8> {
    let mut dist = json!({
        "tarball": "https://evil.example/left-pad-1.0.0.tgz",
        "shasum": "ab".repeat(20),
    });
    dist.as_object_mut()
        .unwrap()
        .extend(integrity_field.as_object().unwrap().clone());
    serde_json::to_vec(&json!({
        "name": "left-pad",
        "dist-tags": {"latest": "1.0.0", "ghost": "9.9.9"},
        "time": {"modified": "2024-01-02T03:04:05.000Z"},
        "versions": {"1.0.0": {
            "name": "left-pad",
            "version": "1.0.0",
            "scripts": {"postinstall": "node x"},
            "dist": dist,
        }},
    }))
    .unwrap()
}

fn good_packument() -> Vec<u8> {
    packument(&json!({"integrity": integrity(TARBALL)}))
}

fn logged(api: &Api, uri: &str, headers: &[(&str, &str)]) -> (Reply, String) {
    let mut all = vec![("authorization", TOKEN_HEADER.as_str())];
    all.extend_from_slice(headers);
    capture_logs(|| api.with_headers(Method::GET, uri, &all, Vec::new()))
}

static TOKEN_HEADER: std::sync::LazyLock<String> = std::sync::LazyLock::new(bearer);

#[test]
fn packument_miss_rewrites_links_to_our_mount_and_hides_foreign_hosts() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));

    // Act
    let (reply, output) = logged(&world.api, URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("vary"), "Accept");
    assert_eq!(reply.header("content-type"), "application/json");
    let doc = reply.json();
    assert_eq!(
        doc["versions"]["1.0.0"]["dist"]["tarball"],
        format!("{PUBLIC_URL}/npm/{NPM_NAME}/left-pad/-/left-pad-1.0.0.tgz")
    );
    assert!(!reply.text().contains("evil.example"), "{}", reply.text());
    assert_eq!(doc["dist-tags"], json!({"latest": "1.0.0"}));
    assert!(output.contains(r#""cache":"miss""#), "{output}");
}

#[test]
fn abbreviated_view_is_derived_locally_from_the_same_cached_document() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));
    world.api.npm_get(URI);

    // Act
    let reply = world.api.with_headers(
        Method::GET,
        URI,
        &[("authorization", &bearer()), ("accept", ABBREVIATED)],
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.header("content-type"), ABBREVIATED);
    assert_eq!(reply.json()["versions"]["1.0.0"]["hasInstallScript"], true);
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn repeated_packument_request_is_a_hit_without_upstream_call() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));
    world.api.npm_get(URI);

    // Act
    let (reply, output) = logged(&world.api, URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(world.upstream.call_count(), 1);
    assert!(output.contains(r#""cache":"hit""#), "{output}");
}

#[test]
fn scoped_packument_goes_upstream_as_one_escaped_segment() {
    // Arrange
    let world = world();
    let doc = serde_json::to_vec(&json!({"name": "@scope/pkg", "versions": {}})).unwrap();
    world.upstream.will("@scope%2fpkg", Step::Found(doc, None));

    // Act
    let reply = world.api.npm_get("/npm/npmjs/@scope%2fpkg");

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(world.upstream.paths(), ["@scope%2fpkg"]);
}

#[test]
fn unknown_package_is_not_found() {
    // Arrange
    let world = world();
    world.upstream.will("left-pad", Step::NotFound);

    // Act
    let reply = world.api.npm_get(URI);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.npm_error(), "не найдено");
}

#[test]
fn tarball_miss_is_verified_by_sha512_and_then_served_from_cache() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));
    world
        .upstream
        .will(TARBALL_PATH, Step::Found(TARBALL.to_vec(), None));
    let first = world.api.npm_get(TARBALL_URI);

    // Act
    let (second, output) = logged(&world.api, TARBALL_URI, &[]);

    // Assert
    assert_eq!(first.body.as_ref(), TARBALL);
    assert_eq!(second.status, StatusCode::OK);
    assert_eq!(second.body.as_ref(), TARBALL);
    assert_eq!(second.header("content-length"), TARBALL.len().to_string());
    assert_eq!(world.upstream.paths(), ["left-pad", TARBALL_PATH]);
    assert!(output.contains(r#""cache":"hit""#), "{output}");
}

#[test]
fn tarball_with_wrong_sha512_is_bad_gateway_and_logged_as_digest_mismatch() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));
    world
        .upstream
        .will(TARBALL_PATH, Step::Found(b"tampered".to_vec(), None));

    // Act
    let (reply, output) = logged(&world.api, TARBALL_URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
    assert_eq!(reply.npm_error(), "upstream недоступен");
    assert!(
        output.contains(r#""upstream_failure":"digest-mismatch""#),
        "{output}"
    );
}

#[test]
fn tampered_tarball_is_not_cached_and_the_next_request_fetches_again() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));
    world
        .upstream
        .will(TARBALL_PATH, Step::Found(b"tampered".to_vec(), None));
    let first = world.api.npm_get(TARBALL_URI);
    world
        .upstream
        .will(TARBALL_PATH, Step::Found(TARBALL.to_vec(), None));

    // Act
    let second = world.api.npm_get(TARBALL_URI);

    // Assert
    assert_eq!(first.status, StatusCode::BAD_GATEWAY);
    assert_eq!(second.body.as_ref(), TARBALL);
}

#[test]
fn sha1_only_version_is_refused_without_asking_upstream_for_the_tarball() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(packument(&json!({})), None));

    // Act
    let (reply, output) = logged(&world.api, TARBALL_URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
    assert_eq!(world.upstream.paths(), ["left-pad"]);
    assert!(
        output.contains(r#""upstream_failure":"weak-integrity""#),
        "{output}"
    );
}

#[test]
fn tarball_of_version_missing_from_packument_is_not_found_without_tarball_request() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));

    // Act
    let reply = world
        .api
        .npm_get("/npm/npmjs/left-pad/-/left-pad-9.9.9.tgz");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.paths(), ["left-pad"]);
}

#[test]
fn packument_is_served_stale_when_upstream_fails_after_expiry() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));
    world.api.npm_get(URI);
    world.clock.advance(EXPIRED_AFTER);
    world.upstream.will("left-pad", Step::Unavailable);

    // Act
    let (reply, output) = logged(&world.api, URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("warning"), "111 - \"Revalidation Failed\"");
    assert!(output.contains(r#""cache":"stale-on-error""#), "{output}");
}

#[test]
fn garbage_from_upstream_is_bad_gateway() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(b"<html>".to_vec(), None));

    // Act
    let reply = world.api.npm_get(URI);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
}

#[test]
fn writes_to_a_proxy_are_method_not_allowed_and_dist_tags_are_not_found() {
    // Arrange
    let world = world();
    let cases = [
        (Method::PUT, URI.to_owned()),
        (Method::DELETE, format!("{URI}/-rev/1-x")),
        (Method::DELETE, format!("{TARBALL_URI}/-rev/1-x")),
        (
            Method::PUT,
            "/npm/npmjs/-/package/left-pad/dist-tags/next".to_owned(),
        ),
        (
            Method::DELETE,
            "/npm/npmjs/-/package/left-pad/dist-tags/next".to_owned(),
        ),
        (
            Method::GET,
            "/npm/npmjs/-/package/left-pad/dist-tags".to_owned(),
        ),
    ];

    // Act
    let statuses = cases.map(|(method, uri)| world.api.npm(method, &uri, Vec::new()).status);

    // Assert
    assert_eq!(
        statuses,
        [
            StatusCode::METHOD_NOT_ALLOWED,
            StatusCode::METHOD_NOT_ALLOWED,
            StatusCode::METHOD_NOT_ALLOWED,
            StatusCode::METHOD_NOT_ALLOWED,
            StatusCode::METHOD_NOT_ALLOWED,
            StatusCode::NOT_FOUND,
        ]
    );
    assert_eq!(world.upstream.call_count(), 0);
}

#[test]
fn proxy_requires_a_bearer_token() {
    // Arrange
    let world = world();

    // Act
    let reply = world.api.with_headers(Method::GET, URI, &[], Vec::new());

    // Assert
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.header("www-authenticate"), "Bearer realm=\"crane\"");
    assert_eq!(world.upstream.call_count(), 0);
}

#[test]
fn proxy_name_wins_over_hosted_repo_parameter() {
    // Arrange
    let world = world();

    // Act
    let reply = world.api.npm_get("/npm/npmjs/-/whoami");

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json(), json!({"username": common::PRINCIPAL}));
}

#[test]
fn hosted_repository_next_to_proxy_keeps_working() {
    // Arrange
    let world = world();
    world.api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Act
    let reply = world.api.npm_get("/npm/web/left-pad");

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}
