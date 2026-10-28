//! Условная ревалидация: If-None-Match, 304 и блокирующая выборка просроченной записи.

use crane_app::proxy::CacheResult;
use futures_executor::block_on;

use crate::common::proxy::upstream::{Call, Step};
use crate::common::proxy::{World, secs};

use super::DOC;

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
            accept: None,
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
