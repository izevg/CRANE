//! Адаптивный TTL: рост при неизменном документе и сброс при изменении.

use crane_app::proxy::CacheResult;

use crate::common::proxy::upstream::Step;
use crate::common::proxy::{World, secs};

use super::DOC;

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
