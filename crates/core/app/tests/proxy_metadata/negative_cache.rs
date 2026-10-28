//! Negative-кэш: 404 хранится и удваивает срок при повторах.

use crane_app::proxy::ProxyError;

use crate::common::proxy::upstream::Step;
use crate::common::proxy::{World, just_before, secs};

use super::DOC;

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
