//! Представления одного пути: `Accept` входит в ключ, поэтому записи, полёты и валидаторы раздельны.

use crate::common::proxy::upstream::{Call, Step};
use crate::common::proxy::{World, secs, with_accept};

use super::DOC;

const JSON: &str = "application/vnd.pypi.simple.v1+json";

#[test]
fn two_accepts_for_one_path_are_fetched_separately() {
    // Arrange
    let world = World::new([
        Step::Found(b"html", Some("h1")),
        Step::Found(b"json", Some("j1")),
    ]);
    let json = with_accept(&world.origin, JSON);
    world.metadata(DOC).unwrap();

    // Act
    let served = world.metadata_via(&json, DOC).unwrap();

    // Assert
    assert_eq!(served.body, "json");
    assert_eq!(
        world.upstream.calls(),
        [
            Call {
                path: DOC.to_owned(),
                accept: None,
                if_none_match: None,
            },
            Call {
                path: DOC.to_owned(),
                accept: Some(JSON),
                if_none_match: None,
            },
        ]
    );
}

#[test]
fn etag_of_one_representation_is_not_sent_for_another() {
    // Arrange
    let world = World::new([
        Step::Found(b"html", Some("h1")),
        Step::Found(b"json", Some("j1")),
        Step::NotModified,
    ]);
    let json = with_accept(&world.origin, JSON);
    world.metadata(DOC).unwrap();
    world.metadata_via(&json, DOC).unwrap();
    world.advance(secs(150));
    let revalidation = world
        .metadata_via(&json, DOC)
        .unwrap()
        .revalidation
        .take()
        .unwrap();

    // Act
    let outcome = futures_executor::block_on(revalidation);

    // Assert
    assert!(outcome.is_ok());
    assert_eq!(
        world.upstream.calls()[2],
        Call {
            path: DOC.to_owned(),
            accept: Some(JSON),
            if_none_match: Some("j1".to_owned()),
        }
    );
}

#[test]
fn negative_entry_of_one_representation_does_not_hide_another() {
    // Arrange
    let world = World::new([Step::NotFound, Step::Found(b"html", None)]);
    let json = with_accept(&world.origin, JSON);
    let _ = world.metadata_via(&json, DOC);

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "html");
    assert_eq!(world.upstream.call_count(), 2);
}
