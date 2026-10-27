//! Proxy-движок, metadata: зоны свежести, адаптивный TTL, stale-if-error, negative-кэш.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::sync::Arc;

use common::proxy::{
    BrokenCache, Call, FakeUpstream, SharedBlobs, SharedLeases, Step, World, just_before, origin,
    path, policy, secs,
};
use crane_app::proxy::{CacheResult, ProxyEngine, ProxyError, ProxyLimits, UpstreamFailure};
use crane_domain::proxy::EgressRefusal;
use crane_memory::ManualClock;
use futures_executor::block_on;

const DOC: &str = "se/rd/serde";

#[test]
fn miss_fetches_upstream_once_and_returns_body() {
    // Arrange
    let world = World::new([Step::Found(b"v1", Some("e1"))]);

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v1");
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn repeat_within_fresh_zone_is_hit_without_upstream_call() {
    // Arrange
    let world = World::new([Step::Found(b"v1", Some("e1"))]);
    world.metadata(DOC).unwrap();
    world.advance(secs(10));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert_eq!(served.age, secs(10));
    assert!(served.revalidation.is_none());
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn refresh_ahead_zone_is_hit_with_revalidation() {
    // Arrange
    let world = World::new([Step::Found(b"v1", Some("e1"))]);
    world.metadata(DOC).unwrap();
    world.advance(secs(95));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert!(served.revalidation.is_some());
}

#[test]
fn zone_after_ttl_is_stale_with_revalidation() {
    // Arrange
    let world = World::new([Step::Found(b"v1", Some("e1"))]);
    world.metadata(DOC).unwrap();
    world.advance(secs(150));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Stale);
    assert_eq!(served.body, "v1");
    assert!(served.revalidation.is_some());
}

#[test]
fn revalidation_sends_if_none_match_with_stored_etag() {
    // Arrange
    let world = World::new([Step::Found(b"v1", Some("e1")), Step::NotModified]);
    world.metadata(DOC).unwrap();
    world.advance(secs(150));
    let revalidation = world.metadata(DOC).unwrap().revalidation.take().unwrap();

    // Act
    let outcome = block_on(revalidation);

    // Assert
    assert!(outcome.is_ok());
    assert_eq!(
        world.upstream.calls()[1],
        Call {
            path: DOC.to_owned(),
            if_none_match: Some("e1".to_owned()),
        }
    );
}

#[test]
fn not_modified_revalidation_renews_the_entry() {
    // Arrange
    let world = World::new([Step::Found(b"v1", Some("e1")), Step::NotModified]);
    world.metadata(DOC).unwrap();
    world.refresh_after(secs(150), DOC);
    world.advance(secs(50));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert_eq!(served.age, secs(50));
    assert!(served.revalidation.is_none());
}

#[test]
fn three_not_modified_in_a_row_double_the_ttl() {
    // Arrange
    let world = World::new([
        Step::Found(b"v1", Some("e1")),
        Step::NotModified,
        Step::NotModified,
        Step::NotModified,
    ]);
    world.metadata(DOC).unwrap();
    world.refresh_after(secs(150), DOC);
    world.refresh_after(secs(150), DOC);
    world.refresh_after(secs(150), DOC);
    world.advance(secs(150));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert!(served.revalidation.is_none());
}

#[test]
fn identical_200_bodies_count_as_unchanged() {
    // Arrange
    let world = World::new([
        Step::Found(b"v1", Some("e1")),
        Step::Found(b"v1", Some("e1")),
        Step::Found(b"v1", Some("e1")),
        Step::Found(b"v1", Some("e1")),
    ]);
    world.metadata(DOC).unwrap();
    world.refresh_after(secs(150), DOC);
    world.refresh_after(secs(150), DOC);
    world.refresh_after(secs(150), DOC);
    world.advance(secs(150));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert!(served.revalidation.is_none());
}

#[test]
fn changed_body_resets_the_adaptive_ttl() {
    // Arrange
    let world = World::new([
        Step::Found(b"v1", Some("e1")),
        Step::NotModified,
        Step::NotModified,
        Step::NotModified,
        Step::Found(b"v2", Some("e2")),
    ]);
    world.metadata(DOC).unwrap();
    world.refresh_after(secs(150), DOC);
    world.refresh_after(secs(150), DOC);
    world.refresh_after(secs(150), DOC);
    world.refresh_after(secs(190), DOC);
    world.advance(secs(150));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v2");
    assert_eq!(served.result, CacheResult::Stale);
}

#[test]
fn expired_entry_is_refetched_blocking_as_miss() {
    // Arrange
    let world = World::new([
        Step::Found(b"v1", Some("e1")),
        Step::Found(b"v2", Some("e2")),
    ]);
    world.metadata(DOC).unwrap();
    world.advance(secs(200));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v2");
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(served.age, secs(0));
    assert!(served.revalidation.is_none());
}

#[test]
fn expired_entry_confirmed_by_not_modified_keeps_old_body_as_miss() {
    // Arrange
    let world = World::new([Step::Found(b"v1", Some("e1")), Step::NotModified]);
    world.metadata(DOC).unwrap();
    world.advance(secs(200));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v1");
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(
        world.upstream.calls()[1].if_none_match.as_deref(),
        Some("e1")
    );
}

#[test]
fn expired_entry_with_upstream_down_serves_stale_on_error() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None), Step::Unavailable]);
    world.metadata(DOC).unwrap();
    world.advance(secs(250));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v1");
    assert_eq!(
        served.result,
        CacheResult::StaleOnError(UpstreamFailure::Unavailable)
    );
    assert_eq!(served.age, secs(250));
}

#[test]
fn stale_if_error_window_ends_exactly_at_ttl_plus_window() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None), Step::Unavailable]);
    world.metadata(DOC).unwrap();
    world.advance(secs(1100));

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::UpstreamUnavailable)));
}

#[test]
fn stale_if_error_window_still_serves_one_millisecond_before_the_end() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None), Step::Unavailable]);
    world.metadata(DOC).unwrap();
    world.advance(just_before(secs(1100)));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(
        served.result,
        CacheResult::StaleOnError(UpstreamFailure::Unavailable)
    );
}

#[test]
fn refusal_within_window_is_reported_alongside_stale_body() {
    // Arrange
    let refusal = EgressRefusal::HostNotAllowed("evil.example".to_owned());
    let world = World::new([Step::Found(b"v1", None), Step::Refused(refusal.clone())]);
    world.metadata(DOC).unwrap();
    world.advance(secs(250));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v1");
    assert_eq!(
        served.result,
        CacheResult::StaleOnError(UpstreamFailure::Refused(refusal))
    );
}

#[test]
fn refusal_without_entry_is_an_error() {
    // Arrange
    let refusal = EgressRefusal::InsecureScheme;
    let world = World::new([Step::Refused(refusal.clone())]);

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::Refused(reason)) if reason == refusal));
}

#[test]
fn oversized_response_is_not_masked_by_stale_entry() {
    // Arrange
    let limits = ProxyLimits {
        max_document_bytes: 2,
        ..ProxyLimits::DEFAULT
    };
    let world = World::with_limits(
        [Step::Found(b"v1", None), Step::Found(b"v2222", None)],
        limits,
    );
    world.metadata(DOC).unwrap();
    world.advance(secs(250));

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::TooLarge { limit: 2 })));
}

#[test]
fn not_found_is_an_error_and_cached_negatively() {
    // Arrange
    let world = World::new([Step::NotFound]);
    let first = world.metadata(DOC);
    world.advance(secs(30));

    // Act
    let second = world.metadata(DOC);

    // Assert
    assert!(matches!(first, Err(ProxyError::NotFound)));
    assert!(matches!(second, Err(ProxyError::NotFound)));
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn negative_entry_still_holds_one_millisecond_before_ttl() {
    // Arrange
    let world = World::new([Step::NotFound, Step::Found(b"v1", None)]);
    let _ = world.metadata(DOC);
    world.advance(just_before(secs(60)));

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::NotFound)));
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn negative_entry_expires_exactly_at_ttl() {
    // Arrange
    let world = World::new([Step::NotFound, Step::Found(b"v1", None)]);
    let _ = world.metadata(DOC);
    world.advance(secs(60));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v1");
    assert_eq!(world.upstream.call_count(), 2);
}

#[test]
fn second_not_found_doubles_negative_ttl() {
    // Arrange
    let world = World::new([Step::NotFound, Step::NotFound, Step::NotFound]);
    let _ = world.metadata(DOC);
    world.advance(secs(60));
    let _ = world.metadata(DOC);
    world.advance(just_before(secs(120)));

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::NotFound)));
    assert_eq!(world.upstream.call_count(), 2);
}

#[test]
fn doubled_negative_ttl_expires_exactly_after_two_ttls() {
    // Arrange
    let world = World::new([Step::NotFound, Step::NotFound, Step::Found(b"v1", None)]);
    let _ = world.metadata(DOC);
    world.advance(secs(60));
    let _ = world.metadata(DOC);
    world.advance(secs(120));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v1");
    assert_eq!(world.upstream.call_count(), 3);
}

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
    assert!(matches!(outcome, Err(ProxyError::UpstreamUnavailable)));
}

#[test]
fn not_modified_without_stored_document_is_upstream_unavailable() {
    // Arrange
    let world = World::new([Step::NotModified]);

    // Act
    let outcome = world.metadata(DOC);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::UpstreamUnavailable)));
}

#[test]
fn failing_cache_degrades_to_direct_upstream_fetch() {
    // Arrange
    let upstream = FakeUpstream::new([Step::Found(b"v1", None)]);
    let clock = Arc::new(ManualClock::new(common::proxy::epoch()));
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

#[test]
fn entry_aged_exactly_ttl_is_stale() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None)]);
    world.metadata(DOC).unwrap();
    world.advance(secs(100));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Stale);
}

#[test]
fn entry_aged_one_millisecond_below_ttl_is_still_hit() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None)]);
    world.metadata(DOC).unwrap();
    world.advance(just_before(secs(100)));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert!(served.revalidation.is_some());
}
