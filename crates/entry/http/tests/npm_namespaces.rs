//! Защита внутренних имён в npm: proxy не отдаёт их из registry.npmjs.org, presence спрашивает его,
//! а публикация без scope отклоняется.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::sync::Arc;

use axum::http::StatusCode;
use common::Api;
use common::names::FixedPresence;
use common::npm::{TARBALL, integrity};
use common::proxy::{FakeUpstream, NpmProxy, Step};
use crane_domain::audit::AuditAction;
use crane_domain::registry::Format;
use crane_ports::proxy::Presence;
use serde_json::json;

struct World {
    api: Api,
    proxy: NpmProxy,
    upstream: FakeUpstream,
}

fn world() -> World {
    let proxy = NpmProxy::new();
    World {
        api: Api::with_npm_proxy(&proxy),
        upstream: proxy.upstream.clone(),
        proxy,
    }
}

/// Packument upstream с корректной суммой tarball.
fn good_packument() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "name": "left-pad",
        "dist-tags": {"latest": "1.0.0"},
        "versions": {"1.0.0": {
            "name": "left-pad",
            "version": "1.0.0",
            "dist": {
                "tarball": "https://registry.npmjs.org/left-pad/-/left-pad-1.0.0.tgz",
                "integrity": integrity(TARBALL),
            },
        }},
    }))
    .unwrap()
}

const INTERNAL_URI: &str = "/npm/npmjs/@acme%2fcore";
const INTERNAL_TARBALL_URI: &str = "/npm/npmjs/@acme/core/-/core-1.0.0.tgz";
const HOSTED_NAME: &str = "@acme/left-pad";
const HOSTED_URI: &str = "/npm/npmjs/@acme%2fleft-pad";
const SCOPED_UPSTREAM_PATH: &str = "@other%2fpkg";

fn shadow_refusals(api: &Api) -> usize {
    api.audit_events()
        .iter()
        .filter(|event| matches!(event.action, AuditAction::ShadowUpstream { .. }))
        .count()
}

#[test]
fn packument_of_name_covered_by_namespace_is_not_found_without_upstream_and_audited() {
    // Arrange
    let world = world();
    world.api.reserve(Format::Npm, "@acme/*");

    // Act
    let reply = world.api.npm_get(INTERNAL_URI);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.call_count(), 0);
    assert_eq!(shadow_refusals(&world.api), 1);
}

#[test]
fn packument_of_name_published_to_hosted_is_not_found_without_upstream() {
    // Arrange
    let world = world();
    world.api.npm_publish(HOSTED_NAME, "1.0.0", TARBALL);

    // Act
    let reply = world.api.npm_get(HOSTED_URI);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.call_count(), 0);
}

#[test]
fn tarball_of_internal_name_is_not_found_without_upstream() {
    // Arrange
    let world = world();
    world.api.reserve(Format::Npm, "@acme/*");

    // Act
    let reply = world.api.npm_get(INTERNAL_TARBALL_URI);

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.call_count(), 0);
    assert_eq!(shadow_refusals(&world.api), 1);
}

#[test]
fn scoped_name_outside_every_namespace_still_goes_to_upstream() {
    // Arrange
    let world = world();
    world.api.reserve(Format::Npm, "@acme/*");
    world.upstream.will(SCOPED_UPSTREAM_PATH, Step::NotFound);

    // Act
    let reply = world.api.npm_get("/npm/npmjs/@other%2fpkg");

    // Assert
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(world.upstream.paths(), [SCOPED_UPSTREAM_PATH]);
}

#[test]
fn presence_of_name_found_upstream_is_public() {
    // Arrange
    let world = world();
    world
        .upstream
        .will("left-pad", Step::Found(good_packument(), None));

    // Act
    let presence = world.proxy.presence(Format::Npm, "left-pad");

    // Assert
    assert_eq!(presence, Presence::Public);
}

#[test]
fn presence_of_name_missing_upstream_is_absent() {
    // Arrange
    let world = world();
    world.upstream.will("left-pad", Step::NotFound);

    // Act
    let presence = world.proxy.presence(Format::Npm, "left-pad");

    // Assert
    assert_eq!(presence, Presence::Absent);
}

#[test]
fn presence_with_failing_upstream_is_unknown() {
    // Arrange
    let world = world();
    world.upstream.will("left-pad", Step::Unavailable);

    // Act
    let presence = world.proxy.presence(Format::Npm, "left-pad");

    // Assert
    assert_eq!(presence, Presence::Unknown);
}

#[test]
fn presence_for_format_without_origin_is_absent_without_upstream_request() {
    // Arrange
    let world = world();

    // Act
    let presence = world.proxy.presence(Format::Cargo, "serde");

    // Assert
    assert_eq!(presence, Presence::Absent);
    assert_eq!(world.upstream.call_count(), 0);
}

#[test]
fn unscoped_name_is_rejected_with_unprocessable_entity() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.npm_publish("left-pad", "1.0.0", TARBALL);

    // Assert
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[test]
fn first_publication_of_name_known_to_public_registry_is_forbidden() {
    // Arrange
    let api = Api::with_upstream(Arc::new(FixedPresence(Presence::Public)));

    // Act
    let reply = api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);

    // Assert
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}

#[test]
fn first_publication_while_public_registry_is_unreachable_is_unavailable() {
    // Arrange
    let api = Api::with_upstream(Arc::new(FixedPresence(Presence::Unknown)));

    // Act
    let reply = api.npm_publish("@acme/left-pad", "1.0.0", TARBALL);

    // Assert
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
}
