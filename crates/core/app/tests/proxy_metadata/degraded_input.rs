//! Некорректные и предельные ответы upstream, отказ кэша.

use std::sync::Arc;

use crane_app::proxy::{CacheResult, ProxyEngine, ProxyError, ProxyLimits, UpstreamFailure};
use crane_memory::ManualClock;
use futures_executor::block_on;

use crate::common::proxy::ports::{BrokenCache, SharedLeases};
use crate::common::proxy::upstream::{FakeUpstream, Step};
use crate::common::proxy::{SharedBlobs, World, origin, path, policy};

use super::DOC;

#[test]
fn document_over_the_limit_is_rejected() {
    // Arrange
    let limits = ProxyLimits {
        max_document_bytes: 4,
        ..ProxyLimits::DEFAULT
    };
    let world = World::with_limits([Step::Found(b"12345", None)], limits);

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::TooLarge { limit: 4 })));
}

#[test]
fn document_exactly_at_the_limit_is_accepted() {
    // Arrange
    let limits = ProxyLimits {
        max_document_bytes: 4,
        ..ProxyLimits::DEFAULT
    };
    let world = World::with_limits([Step::Found(b"1234", None)], limits);

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "1234");
}

#[test]
fn broken_body_stream_is_upstream_unavailable() {
    // Arrange
    let world = World::new([Step::BrokenBody]);

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(
        outcome,
        Err(ProxyError::Upstream(UpstreamFailure::Unavailable))
    ));
}

#[test]
fn not_modified_without_stored_document_is_upstream_unavailable() {
    // Arrange
    let world = World::new([Step::NotModified]);

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(
        outcome,
        Err(ProxyError::Upstream(UpstreamFailure::Unavailable))
    ));
}

#[test]
fn failing_cache_degrades_to_direct_upstream_fetch() {
    // Arrange
    let upstream = FakeUpstream::new([Step::Found(b"v1", None)]);
    let clock = Arc::new(ManualClock::new(crate::common::proxy::epoch()));
    let engine = ProxyEngine::new(
        upstream.clone(),
        BrokenCache,
        SharedLeases::new(Arc::clone(&clock)),
        SharedBlobs::default(),
        clock,
        ProxyLimits::DEFAULT,
    );

    // Act
    let served = block_on(engine.metadata(&origin(), &path(DOC), &policy())).unwrap();

    // Assert
    assert_eq!(served.body, "v1");
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(upstream.call_count(), 1);
}
