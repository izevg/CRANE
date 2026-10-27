//! Proxy-движок: ограничение запросов upstream (429/403) режет окно stale-if-error до суток.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::time::Duration;

use bytes::Bytes;
use common::proxy::upstream::Step;
use common::proxy::{World, just_before, path, policy, secs};
use crane_app::proxy::{CacheResult, ProxyError, Served, UpstreamFailure};
use crane_domain::proxy::MetadataPolicy;

const DOC: &str = "se/rd/serde";

/// Как `policy()`, но stale-if-error неделя: потолок 24 ч при ограничении запросов достижим.
fn week_policy() -> MetadataPolicy {
    MetadataPolicy {
        stale_if_error: Duration::from_hours(168),
        ..policy()
    }
}

fn metadata_with(world: &World) -> Result<Served<Bytes>, ProxyError> {
    futures_executor::block_on(
        world
            .engine
            .metadata(&world.origin, &path(DOC), &week_policy()),
    )
}

#[test]
fn throttled_entry_within_a_day_past_ttl_serves_stale_on_error() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None), Step::Throttled]);
    metadata_with(&world).unwrap();
    world.advance(just_before(secs(100 + 86_400)));

    // Act
    let served = metadata_with(&world).unwrap();

    // Assert
    assert_eq!(
        served.result,
        CacheResult::StaleOnError(UpstreamFailure::Throttled)
    );
}

#[test]
fn throttled_entry_past_a_day_is_not_served_despite_week_long_stale_if_error() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None), Step::Throttled]);
    metadata_with(&world).unwrap();
    world.advance(secs(100 + 86_400));

    // Act
    let outcome = metadata_with(&world);

    // Assert
    assert!(matches!(
        outcome,
        Err(ProxyError::Upstream(UpstreamFailure::Throttled))
    ));
}

#[test]
fn unavailable_entry_past_a_day_is_still_served_within_stale_if_error() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None), Step::Unavailable]);
    metadata_with(&world).unwrap();
    world.advance(secs(100 + 86_400));

    // Act
    let served = metadata_with(&world).unwrap();

    // Assert
    assert_eq!(
        served.result,
        CacheResult::StaleOnError(UpstreamFailure::Unavailable)
    );
}
