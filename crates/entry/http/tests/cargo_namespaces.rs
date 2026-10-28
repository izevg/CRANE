//! Защита внутренних имён в cargo: proxy не отдаёт их из crates.io, presence спрашивает crates.io.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use axum::http::StatusCode;
use common::cargo::sha256_hex;
use common::proxy::{FakeUpstream, Proxy, Step};
use common::{Api, REPO};
use crane_domain::audit::AuditAction;
use crane_domain::registry::Format;
use crane_ports::proxy::Presence;

const INDEX_PATH: &str = "se/rd/serde";
const INDEX_URI: &str = "/cargo/crates-io/index/se/rd/serde";
const CRATE: &[u8] = b"\x1f\x8b fake crate bytes";

struct World {
    api: Api,
    proxy: Proxy,
    upstream: FakeUpstream,
}

fn world() -> World {
    let proxy = Proxy::new();
    World {
        api: Api::with_proxy(&proxy),
        upstream: proxy.upstream.clone(),
        proxy,
    }
}

fn serde_index() -> Vec<u8> {
    format!(
        r#"{{"name":"serde","vers":"1.0.0","deps":[],"cksum":"{}","features":{{}},"yanked":false}}{}"#,
        sha256_hex(CRATE),
        "\n"
    )
    .into_bytes()
}

fn download_uri(name: &str, version: &str) -> String {
    format!("/cargo/crates-io/api/v1/crates/{name}/{version}/download")
}

const INTERNAL_INDEX_URI: &str = "/cargo/crates-io/index/ac/me/acme-core";
const HOSTED_INDEX_URI: &str = "/cargo/crates-io/index/de/mo/demo-crate";

fn shadow_refusals(api: &Api) -> usize {
    api.audit_events()
        .iter()
        .filter(|event| matches!(event.action, AuditAction::ShadowUpstream { .. }))
        .count()
}

#[test]
fn index_of_name_covered_by_namespace_is_not_found_without_upstream_and_audited() {
    // Arrange
    let world = world();
    world.api.reserve(Format::Cargo, "acme-*");

    // Act
    let reply = world.api.get(INTERNAL_INDEX_URI);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.detail(), "не найдено");
    assert_eq!(world.upstream.call_count(), 0);
    assert_eq!(shadow_refusals(&world.api), 1);
}

#[test]
fn index_of_name_published_to_hosted_is_not_found_without_upstream() {
    // Arrange
    let world = world();
    world.api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Act
    let reply = world.api.get(HOSTED_INDEX_URI);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.call_count(), 0);
    assert_eq!(shadow_refusals(&world.api), 1);
}

#[test]
fn download_of_internal_name_is_not_found_without_upstream() {
    // Arrange
    let world = world();
    world.api.reserve(Format::Cargo, "acme-*");

    // Act
    let reply = world.api.get(&download_uri("acme-core", "1.0.0"));

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.call_count(), 0);
    assert_eq!(shadow_refusals(&world.api), 1);
}

#[test]
fn name_outside_every_namespace_still_goes_to_upstream() {
    // Arrange
    let world = world();
    world.api.reserve(Format::Cargo, "acme-*");
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), None));

    // Act
    let reply = world.api.get(INDEX_URI);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(world.upstream.paths(), [INDEX_PATH]);
}

#[test]
fn presence_of_name_found_upstream_is_public() {
    // Arrange
    let world = world();
    world
        .upstream
        .will(INDEX_PATH, Step::Found(serde_index(), None));

    // Act
    let presence = world.proxy.presence(Format::Cargo, "serde");

    // Assert
    assert_eq!(presence, Presence::Public);
}

#[test]
fn presence_of_name_missing_upstream_is_absent() {
    // Arrange
    let world = world();
    world.upstream.will(INDEX_PATH, Step::NotFound);

    // Act
    let presence = world.proxy.presence(Format::Cargo, "serde");

    // Assert
    assert_eq!(presence, Presence::Absent);
}

#[test]
fn presence_with_failing_upstream_is_unknown() {
    // Arrange
    let world = world();
    world.upstream.will(INDEX_PATH, Step::Unavailable);

    // Act
    let presence = world.proxy.presence(Format::Cargo, "serde");

    // Assert
    assert_eq!(presence, Presence::Unknown);
}

#[test]
fn presence_for_other_format_is_absent_without_upstream_request() {
    // Arrange
    let world = world();

    // Act
    let presence = world.proxy.presence(Format::Npm, "serde");

    // Assert
    assert_eq!(presence, Presence::Absent);
    assert_eq!(world.upstream.call_count(), 0);
}
