//! Proxy-репозиторий crates.io через HTTP: фейковый upstream, memory-адаптеры, сети нет.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::http::{Method, StatusCode};
use common::proxy::{FakeUpstream, NAME, Proxy, Step};
use common::{Api, PUBLIC_URL, REPO, Reply, SharedBuf, TOKEN, sha256_hex};
use crane_domain::proxy::EgressRefusal;
use crane_memory::ManualClock;
use tracing_subscriber::fmt;

const INDEX_PATH: &str = "se/rd/serde";
const INDEX_URI: &str = "/cargo/crates-io/index/se/rd/serde";
const CRATE: &[u8] = b"\x1f\x8b fake crate bytes";
const ETAG: &str = "\"upstream-etag\"";
/// Индекс короче 5 минут свежести и длиннее окна stale-while-revalidate после неё.
const STALE_AFTER: Duration = Duration::from_secs(400);
const EXPIRED_AFTER: Duration = Duration::from_secs(700);

struct World {
    api: Api,
    upstream: FakeUpstream,
    clock: Arc<ManualClock>,
}

fn world() -> World {
    let proxy = Proxy::new();
    World {
        api: Api::with_proxy_repos(vec![(NAME.to_owned(), proxy.router)]),
        upstream: proxy.upstream,
        clock: proxy.clock,
    }
}

fn index_line(name: &str, vers: &str, cksum: &str) -> String {
    format!(
        r#"{{"name":"{name}","vers":"{vers}","deps":[],"cksum":"{cksum}","features":{{}},"yanked":false}}{}"#,
        "\n"
    )
}

fn serde_index() -> Vec<u8> {
    index_line("serde", "1.0.0", &sha256_hex(CRATE)).into_bytes()
}

fn download_uri(name: &str, version: &str) -> String {
    format!("/cargo/{NAME}/api/v1/crates/{name}/{version}/download")
}

/// Один запрос с токеном под перехватом JSON-журнала, как его пишет сервер.
fn logged(api: &Api, uri: &str, headers: &[(&str, &str)]) -> (Reply, String) {
    let buffer = SharedBuf::default();
    let subscriber = fmt()
        .json()
        .flatten_event(true)
        .with_writer(buffer.clone())
        .finish();
    let mut all = vec![("authorization", TOKEN)];
    all.extend_from_slice(headers);
    let reply = tracing::subscriber::with_default(subscriber, || {
        api.with_headers(Method::GET, uri, &all, Vec::new())
    });
    (reply, buffer.text())
}

#[test]
fn index_miss_is_served_byte_for_byte_with_etag_age_and_miss_in_log() {
    // Arrange
    let world = world();
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), Some(ETAG)));

    // Act
    let (reply, output) = logged(&world.api, INDEX_URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body.as_ref(), serde_index().as_slice());
    assert_eq!(
        reply.header("etag"),
        format!("\"{}\"", sha256_hex(&serde_index()))
    );
    assert_eq!(reply.header("age"), "0");
    assert_eq!(reply.header("content-type"), "text/plain; charset=utf-8");
    assert!(output.contains(r#""cache":"miss""#), "{output}");
}

#[test]
fn repeated_index_request_is_a_hit_without_upstream_call() {
    // Arrange
    let world = world();
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), Some(ETAG)));
    world.api.get(INDEX_URI);

    // Act
    let (reply, output) = logged(&world.api, INDEX_URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(world.upstream.call_count(), 1);
    assert!(output.contains(r#""cache":"hit""#), "{output}");
}

#[test]
fn matching_if_none_match_gets_not_modified() {
    // Arrange
    let world = world();
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), Some(ETAG)));
    let etag = world.api.get(INDEX_URI).header("etag").to_owned();

    // Act
    let (reply, _) = logged(&world.api, INDEX_URI, &[("if-none-match", etag.as_str())]);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_MODIFIED);
    assert!(reply.body.is_empty());
}

#[test]
fn download_fetches_archive_by_index_spelling_and_checks_sum() {
    // Arrange
    let world = world();
    let index = index_line("Inflector", "0.11.4", &sha256_hex(CRATE)).into_bytes();
    world
        .upstream
        .will("in/fl/inflector", Step::Found(index, None));
    let archive = "crates/Inflector/Inflector-0.11.4.crate";
    world
        .upstream
        .will(archive, Step::Found(CRATE.to_vec(), None));

    // Act
    let (reply, output) = logged(&world.api, &download_uri("Inflector", "0.11.4"), &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body.as_ref(), CRATE);
    assert_eq!(reply.header("content-length"), CRATE.len().to_string());
    assert_eq!(world.upstream.paths(), ["in/fl/inflector", archive]);
    assert!(output.contains(r#""cache":"miss""#), "{output}");
}

#[test]
fn download_of_version_missing_from_index_is_not_found_without_archive_request() {
    // Arrange
    let world = world();
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), None));

    // Act
    let (reply, _) = logged(&world.api, &download_uri("serde", "9.9.9"), &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.paths(), [INDEX_PATH]);
}

#[test]
fn download_of_unknown_crate_is_not_found() {
    // Arrange
    let world = world();
    world.upstream.will(INDEX_PATH, Step::NotFound);

    // Act
    let (reply, _) = logged(&world.api, &download_uri("serde", "1.0.0"), &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.paths(), [INDEX_PATH]);
}

#[test]
fn archive_with_wrong_sum_is_bad_gateway_and_logged_as_digest_mismatch() {
    // Arrange
    let world = world();
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), None));
    world.upstream.will(
        "crates/serde/serde-1.0.0.crate",
        Step::Found(b"tampered".to_vec(), None),
    );

    // Act
    let (reply, output) = logged(&world.api, &download_uri("serde", "1.0.0"), &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
    assert_eq!(reply.detail(), "upstream недоступен");
    assert!(
        output.contains(r#""upstream_failure":"digest-mismatch""#),
        "{output}"
    );
    assert!(output.contains(&sha256_hex(CRATE)), "{output}");
}

#[test]
fn refused_upstream_is_bad_gateway_and_logged_with_reason() {
    // Arrange
    let world = world();
    let refusal = EgressRefusal::HostNotAllowed("evil.example".to_owned());
    world.upstream.will(INDEX_PATH, Step::Refused(refusal));

    // Act
    let (reply, output) = logged(&world.api, INDEX_URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
    assert_eq!(reply.detail(), "upstream недоступен");
    assert!(
        output.contains(r#""upstream_failure":"refused""#),
        "{output}"
    );
    assert!(output.contains("evil.example"), "{output}");
}

#[test]
fn index_is_served_stale_when_upstream_fails_after_expiry() {
    // Arrange
    let world = world();
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), None));
    world.api.get(INDEX_URI);
    world.clock.advance(EXPIRED_AFTER);
    world.upstream.will(INDEX_PATH, Step::Unavailable);

    // Act
    let (reply, output) = logged(&world.api, INDEX_URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("warning"), "111 - \"Revalidation Failed\"");
    assert_eq!(reply.header("age"), EXPIRED_AFTER.as_secs().to_string());
    assert!(output.contains(r#""cache":"stale-on-error""#), "{output}");
    assert!(
        output.contains(r#""upstream_failure":"unavailable""#),
        "{output}"
    );
}

#[test]
fn stale_index_is_served_with_warning_and_revalidated_in_background() {
    // Arrange
    let world = world();
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), Some(ETAG)));
    world.api.get(INDEX_URI);
    world.clock.advance(STALE_AFTER);
    world.upstream.will(INDEX_PATH, Step::NotFound);

    // Act
    let (reply, output) = logged(&world.api, INDEX_URI, &[]);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("warning"), "110 - \"Response is Stale\"");
    assert!(output.contains(r#""cache":"stale""#), "{output}");
    world.api.wait_until(|| world.upstream.call_count() == 2);
    assert_eq!(world.upstream.last_condition(), Some(ETAG.to_owned()));
}

#[test]
fn proxy_name_wins_over_hosted_repo_parameter() {
    // Arrange
    let world = world();

    // Act
    let reply = world.api.get("/cargo/crates-io/index/config.json");

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.json()["dl"],
        format!("{PUBLIC_URL}/cargo/crates-io/api/v1/crates")
    );
}

#[test]
fn unknown_path_under_proxy_name_gets_cargo_shaped_not_found() {
    // Arrange
    let world = world();

    // Act
    let reply = world.api.get("/cargo/crates-io/nonsense");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.header("content-type"), "application/json");
    assert_eq!(reply.detail(), "не найдено");
}

#[test]
fn hosted_repository_next_to_proxy_keeps_working() {
    // Arrange
    let world = world();
    world.api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Act
    let reply = world.api.index_text("demo-crate");

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains(&sha256_hex(CRATE)));
}

#[test]
fn publish_to_proxy_is_method_not_allowed() {
    // Arrange
    let world = world();
    let body = common::publish_body("demo-crate", "1.0.0", CRATE);

    // Act
    let reply = world.api.authed(
        Method::PUT,
        &format!("/cargo/{NAME}/api/v1/crates/new"),
        body,
    );

    // Assert
    assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(reply.header("content-type"), "application/json");
}

#[test]
fn proxy_routes_require_a_token() {
    // Arrange
    let world = world();
    let routes = [
        (Method::GET, "/cargo/crates-io/index/config.json"),
        (Method::GET, INDEX_URI),
        (
            Method::GET,
            "/cargo/crates-io/api/v1/crates/serde/1.0.0/download",
        ),
        (Method::PUT, "/cargo/crates-io/api/v1/crates/new"),
        (Method::GET, "/cargo/crates-io/nonsense"),
    ];

    // Act
    let statuses =
        routes.map(|(method, uri)| world.api.with_headers(method, uri, &[], Vec::new()).status);

    // Assert
    assert_eq!(statuses, [StatusCode::UNAUTHORIZED; 5]);
    assert_eq!(world.upstream.call_count(), 0);
}
