//! Срок, объявленный upstream (`Cache-Control: max-age`), и его сохранение между ревалидациями.

use crane_app::proxy::CacheResult;
use crane_ports::proxy::{Cached, CachedState};
use futures_executor::block_on;

use crane_ports::proxy::ProxyCache;

use crate::common::proxy::upstream::Step;
use crate::common::proxy::{World, key, secs};

use super::DOC;

#[test]
fn upstream_max_age_is_stored_with_the_document() {
    // Arrange
    let world = World::new([Step::FoundWithMaxAge(b"v1", Some("e1"), secs(600))]);

    // Act
    let served = world.metadata(DOC);

    // Assert
    let entry = block_on(world.cache.get(&key(DOC))).unwrap();
    assert!(served.is_ok());
    assert!(matches!(
        entry,
        Some(Cached {
            state: CachedState::Document {
                max_age: Some(max_age),
                ..
            },
            ..
        }) if max_age == secs(600)
    ));
}

#[test]
fn not_modified_keeps_the_max_age_of_the_stored_document() {
    // Arrange
    let world = World::new([
        Step::FoundWithMaxAge(b"v1", Some("e1"), secs(600)),
        Step::NotModified,
    ]);
    world.metadata(DOC).unwrap();
    world.advance(secs(700));

    // Act
    let served = world.metadata(DOC);

    // Assert
    let entry = block_on(world.cache.get(&key(DOC))).unwrap();
    assert!(served.is_ok());
    assert!(matches!(
        entry,
        Some(Cached {
            state: CachedState::Document {
                max_age: Some(max_age),
                unchanged: 1,
                ..
            },
            ..
        }) if max_age == secs(600)
    ));
}

#[test]
fn not_modified_replaces_the_stored_max_age_with_its_own() {
    // Arrange
    let world = World::new([
        Step::FoundWithMaxAge(b"v1", Some("e1"), secs(600)),
        Step::NotModifiedWithMaxAge(secs(700)),
    ]);
    world.metadata(DOC).unwrap();
    world.advance(secs(700));

    // Act
    let served = world.metadata(DOC);

    // Assert
    let entry = block_on(world.cache.get(&key(DOC))).unwrap();
    assert!(served.is_ok());
    assert!(matches!(
        entry,
        Some(Cached {
            state: CachedState::Document {
                max_age: Some(max_age),
                ..
            },
            ..
        }) if max_age == secs(700)
    ));
}

#[test]
fn upstream_max_age_beyond_ttl_keeps_the_document_a_hit_without_upstream_call() {
    // Arrange
    let world = World::new([Step::FoundWithMaxAge(b"v1", Some("e1"), secs(600))]);
    world.metadata(DOC).unwrap();
    world.advance(secs(420)); // ttl политики 100 с: без подсказки запись давно просрочена

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert_eq!(world.upstream.call_count(), 1);
}
