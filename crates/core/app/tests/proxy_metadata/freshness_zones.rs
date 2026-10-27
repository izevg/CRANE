//! Зоны свежести: хит, обновление наперёд, stale и точные границы зон.

use crane_app::proxy::CacheResult;

use crate::common::proxy::upstream::Step;
use crate::common::proxy::{World, just_before, secs};

use super::DOC;

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
