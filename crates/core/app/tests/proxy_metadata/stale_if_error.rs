//! Stale-if-error: окно прощения сбоев upstream и отказы egress.

use crane_app::proxy::{CacheResult, ProxyError, ProxyLimits, UpstreamFailure};
use crane_domain::proxy::EgressRefusal;

use crate::common::proxy::upstream::Step;
use crate::common::proxy::{World, just_before, secs};

use super::DOC;

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
    assert!(matches!(
        outcome,
        Err(ProxyError::Upstream(UpstreamFailure::Unavailable))
    ));
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
    assert!(
        matches!(outcome, Err(ProxyError::Upstream(UpstreamFailure::Refused(reason))) if reason == refusal)
    );
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
