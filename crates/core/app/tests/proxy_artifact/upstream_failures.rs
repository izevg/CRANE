//! Сбои upstream для артефакта: без stale-запасного ответа.

use crane_app::proxy::{ProxyError, UpstreamFailure};
use crane_domain::proxy::EgressRefusal;

use crate::common::proxy::World;
use crate::common::proxy::upstream::Step;

use super::CRATE;

#[test]
fn unavailable_upstream_fails_without_stale_fallback() {
    // Arrange
    let world = World::new([Step::Unavailable]);

    // Act
    let outcome = world.artifact(CRATE, None);

    // Assert
    assert!(matches!(
        outcome,
        Err(ProxyError::Upstream(UpstreamFailure::Unavailable))
    ));
}

#[test]
fn refused_upstream_is_reported() {
    // Arrange
    let refusal = EgressRefusal::HostNotAllowed("evil.example".to_owned());
    let world = World::new([Step::Refused(refusal.clone())]);

    // Act
    let outcome = world.artifact(CRATE, None);

    // Assert
    assert!(
        matches!(outcome, Err(ProxyError::Upstream(UpstreamFailure::Refused(reason))) if reason == refusal)
    );
}

#[test]
fn broken_artifact_stream_is_upstream_unavailable() {
    // Arrange
    let world = World::new([Step::BrokenBody]);

    // Act
    let outcome = world.artifact(CRATE, None);

    // Assert
    assert!(matches!(
        outcome,
        Err(ProxyError::Upstream(UpstreamFailure::Unavailable))
    ));
}
