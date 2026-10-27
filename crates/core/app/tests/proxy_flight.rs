//! Proxy-движок, singleflight: схлопывание в процессе, lease между репликами, отказ координатора.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::pin::pin;

use common::proxy::{FakeUpstream, Step, World, path, policy, poll_once, secs};
use crane_app::proxy::{CacheResult, ProxyError};
use futures_executor::block_on;
use futures_util::future::join;

const DOC: &str = "se/rd/serde";

#[test]
fn concurrent_requests_for_one_key_share_a_single_upstream_call() {
    // Arrange
    let (upstream, gate) = FakeUpstream::gated([Step::Found(b"v1", None)]);
    let world = World::without_leases(upstream);
    let (doc, policy) = (path(DOC), policy());
    let mut first = pin!(world.engine.metadata(&world.origin, &doc, &policy));
    let mut second = pin!(world.engine.metadata(&world.origin, &doc, &policy));
    let _ = poll_once(first.as_mut());
    let _ = poll_once(second.as_mut());
    gate.open();

    // Act
    let (first, second) = block_on(join(first, second));

    // Assert
    assert_eq!(first.unwrap().body, "v1");
    assert_eq!(second.unwrap().body, "v1");
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn different_keys_are_fetched_separately() {
    // Arrange
    let world = World::without_leases(FakeUpstream::new([
        Step::Found(b"a", None),
        Step::Found(b"b", None),
    ]));
    let (one, two, policy) = (path("a/b"), path("c/d"), policy());
    let first = world.engine.metadata(&world.origin, &one, &policy);
    let second = world.engine.metadata(&world.origin, &two, &policy);

    // Act
    let (first, second) = block_on(join(first, second));

    // Assert
    assert_eq!(first.unwrap().body, "a");
    assert_eq!(second.unwrap().body, "b");
    let paths: Vec<_> = world
        .upstream
        .calls()
        .into_iter()
        .map(|call| call.path)
        .collect();
    assert_eq!(paths, ["a/b", "c/d"]);
}

#[test]
fn finished_flight_is_forgotten_so_expired_entry_is_fetched_again() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None), Step::Found(b"v2", None)]);
    world.metadata(DOC).unwrap();
    world.advance(secs(250));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v2");
    assert_eq!(world.upstream.call_count(), 2);
}

#[test]
fn second_replica_serves_the_entry_cached_by_the_leader() {
    // Arrange
    let (upstream_a, gate) = FakeUpstream::gated([Step::Found(b"v1", Some("e1"))]);
    let world = World::with_upstream(upstream_a.clone());
    let upstream_b = FakeUpstream::new([]);
    let replica = world.replica(upstream_b.clone());
    let (doc, policy) = (path(DOC), policy());
    let mut leader = pin!(world.engine.metadata(&world.origin, &doc, &policy));
    let mut follower = pin!(replica.metadata(&world.origin, &doc, &policy));
    let _ = poll_once(leader.as_mut());
    let _ = poll_once(follower.as_mut());
    gate.open();

    // Act
    let (leader, follower) = block_on(join(leader, follower));

    // Assert
    assert_eq!(leader.unwrap().body, "v1");
    let follower = follower.unwrap();
    assert_eq!(follower.body, "v1");
    assert_eq!(follower.result, CacheResult::Miss);
    assert_eq!(upstream_a.call_count(), 1);
    assert_eq!(upstream_b.call_count(), 0);
}

#[test]
fn follower_fetches_itself_when_the_leader_left_nothing_in_cache() {
    // Arrange
    let (upstream_a, gate) = FakeUpstream::gated([Step::Unavailable]);
    let world = World::with_upstream(upstream_a);
    let upstream_b = FakeUpstream::new([Step::Found(b"v1", None)]);
    let replica = world.replica(upstream_b.clone());
    let (doc, policy) = (path(DOC), policy());
    let mut leader = pin!(world.engine.metadata(&world.origin, &doc, &policy));
    let mut follower = pin!(replica.metadata(&world.origin, &doc, &policy));
    let _ = poll_once(leader.as_mut());
    let _ = poll_once(follower.as_mut());
    gate.open();

    // Act
    let (leader, follower) = block_on(join(leader, follower));

    // Assert
    assert!(matches!(leader, Err(ProxyError::UpstreamUnavailable)));
    assert_eq!(follower.unwrap().body, "v1");
    assert_eq!(upstream_b.call_count(), 1);
}

#[test]
fn dropped_revalidation_never_calls_upstream() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None)]);
    world.metadata(DOC).unwrap();
    world.advance(secs(150));
    let stale = world.metadata(DOC).unwrap();

    // Act
    drop(stale);

    // Assert
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn unreachable_lease_coordinator_does_not_fail_the_request() {
    // Arrange
    let world = World::without_leases(FakeUpstream::new([Step::Found(b"v1", None)]));

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn leader_releases_its_lease_after_a_successful_fetch() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None)]);

    // Act
    let served = world.metadata(DOC);

    // Assert
    assert!(served.is_ok());
    assert_eq!((world.leases.led(), world.leases.released()), (1, 1));
}

#[test]
fn leader_releases_its_lease_after_a_failed_fetch() {
    // Arrange
    let world = World::new([Step::Unavailable]);

    // Act
    let served = world.metadata(DOC);

    // Assert
    assert!(served.is_err());
    assert_eq!((world.leases.led(), world.leases.released()), (1, 1));
}
