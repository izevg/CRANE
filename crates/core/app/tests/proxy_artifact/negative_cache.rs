//! Negative-кэш артефакта: 404 хранится и удваивает срок.

use crane_app::proxy::{CacheResult, ProxyError};

use crate::common::proxy::upstream::Step;
use crate::common::proxy::{World, just_before, secs};

use super::CRATE;

#[test]
fn not_found_is_cached_negatively() {
    // Arrange
    let world = World::new([Step::NotFound]);
    let _ = world.artifact(CRATE, None);
    world.advance(secs(299));

    // Act
    let outcome = world.artifact(CRATE, None);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::NotFound)));
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn negative_entry_expires_at_artifact_ttl() {
    // Arrange
    let world = World::new([Step::NotFound, Step::Found(b"hello", None)]);
    let _ = world.artifact(CRATE, None);
    world.advance(secs(300));

    // Act
    let served = world.artifact(CRATE, None).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(world.upstream.call_count(), 2);
}

#[test]
fn repeated_not_found_doubles_negative_ttl() {
    // Arrange
    let world = World::new([Step::NotFound, Step::NotFound, Step::NotFound]);
    let _ = world.artifact(CRATE, None);
    world.advance(secs(300));
    let _ = world.artifact(CRATE, None);
    world.advance(just_before(secs(600)));

    // Act
    let outcome = world.artifact(CRATE, None);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::NotFound)));
    assert_eq!(world.upstream.call_count(), 2);
}
