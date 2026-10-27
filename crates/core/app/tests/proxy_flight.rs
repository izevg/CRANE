//! Proxy-движок, singleflight: схлопывание в процессе, lease между репликами, отказ координатора.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::pin::pin;
use std::task::Poll;

use common::hello_sha256;
use common::proxy::{FakeUpstream, Step, World, bytes_of, key, path, policy, poll_once, secs};
use crane_app::proxy::{CacheResult, ProxyError, UpstreamFailure};
use crane_domain::proxy::NegativePolicy;
use crane_domain::registry::{Checksum, Sha256};
use crane_ports::proxy::{Cached, CachedState, Claim, FetchLeases, ProxyCache};
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
    assert!(matches!(
        leader,
        Err(ProxyError::Upstream(UpstreamFailure::Unavailable))
    ));
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
fn key_is_free_for_the_next_leader_after_a_successful_fetch() {
    // Arrange
    let world = World::new([Step::Found(b"v1", None)]);
    world.metadata(DOC).unwrap();
    let wanted = key(DOC);

    // Act
    let claim = poll_once(pin!(world.leases.claim(&wanted, secs(30), secs(25))));

    // Assert
    assert!(matches!(claim, Poll::Ready(Ok(Claim::Lead(_)))));
}

#[test]
fn key_is_free_for_the_next_leader_after_a_failed_fetch() {
    // Arrange
    let world = World::new([Step::Unavailable]);
    let _ = world.metadata(DOC);
    let wanted = key(DOC);

    // Act
    let claim = poll_once(pin!(world.leases.claim(&wanted, secs(30), secs(25))));

    // Assert
    assert!(matches!(claim, Poll::Ready(Ok(Claim::Lead(_)))));
}

#[test]
fn abandoned_flight_is_not_resumed_by_the_next_request() {
    // Arrange
    let (upstream, gate) = FakeUpstream::gated([Step::Found(b"v2", None)]);
    let world = World::without_leases(upstream);
    let (doc, policy) = (path(DOC), policy());
    let mut abandoned = Box::pin(world.engine.metadata(&world.origin, &doc, &policy));
    let _ = poll_once(abandoned.as_mut());
    drop(abandoned);
    gate.open();

    // Act
    let served = world.metadata(DOC).unwrap();

    // Assert
    assert_eq!(served.body, "v2");
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(world.upstream.call_count(), 2);
}

#[test]
fn joiner_with_foreign_expected_digest_is_rejected_by_shared_flight() {
    // Arrange
    let (upstream, gate) = FakeUpstream::gated([Step::Found(b"hello", None)]);
    let world = World::without_leases(upstream);
    let (blob, negative) = (path("crates/a.crate"), NegativePolicy::ARTIFACT);
    let foreign = Checksum::Sha256(Sha256::from([0xab; 32]));
    let mut leader = pin!(world.engine.artifact(&world.origin, &blob, None, &negative));
    let mut joiner = pin!(
        world
            .engine
            .artifact(&world.origin, &blob, Some(foreign), &negative)
    );
    let _ = poll_once(leader.as_mut());
    let _ = poll_once(joiner.as_mut());
    gate.open();

    // Act
    let (leader, joiner) = block_on(join(leader, joiner));

    // Assert
    assert_eq!(leader.unwrap().body.digests.sha256, hello_sha256());
    assert!(matches!(
        joiner,
        Err(ProxyError::DigestMismatch { expected, actual }) if *expected == foreign && *actual == Checksum::Sha256(hello_sha256())
    ));
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn follower_serves_the_artifact_stored_by_the_leader() {
    // Arrange
    let (upstream_a, gate) = FakeUpstream::gated([Step::Found(b"hello", None)]);
    let world = World::with_upstream(upstream_a);
    let upstream_b = FakeUpstream::new([]);
    let replica = world.replica(upstream_b.clone());
    let (blob, negative) = (path("crates/a.crate"), NegativePolicy::ARTIFACT);
    let mut leader = pin!(world.engine.artifact(&world.origin, &blob, None, &negative));
    let mut follower = pin!(replica.artifact(
        &world.origin,
        &blob,
        Some(Checksum::Sha256(hello_sha256())),
        &negative
    ));
    let _ = poll_once(leader.as_mut());
    let _ = poll_once(follower.as_mut());
    gate.open();

    // Act
    let (_, follower) = block_on(join(leader, follower));

    // Assert
    let follower = follower.unwrap();
    assert_eq!(follower.result, CacheResult::Miss);
    assert_eq!(bytes_of(follower.body), b"hello");
    assert_eq!(upstream_b.call_count(), 0);
}

#[test]
fn follower_with_foreign_expected_digest_ignores_the_leaders_entry() {
    // Arrange
    let (upstream_a, gate) = FakeUpstream::gated([Step::Found(b"hello", None)]);
    let world = World::with_upstream(upstream_a);
    let upstream_b = FakeUpstream::new([Step::Found(b"hello", None)]);
    let replica = world.replica(upstream_b.clone());
    let (blob, negative) = (path("crates/a.crate"), NegativePolicy::ARTIFACT);
    let foreign = Checksum::Sha256(Sha256::from([0xab; 32]));
    let mut leader = pin!(world.engine.artifact(&world.origin, &blob, None, &negative));
    let mut follower = pin!(replica.artifact(&world.origin, &blob, Some(foreign), &negative));
    let _ = poll_once(leader.as_mut());
    let _ = poll_once(follower.as_mut());
    gate.open();

    // Act
    let (_, follower) = block_on(join(leader, follower));

    // Assert
    assert!(matches!(follower, Err(ProxyError::DigestMismatch { .. })));
    assert_eq!(upstream_b.call_count(), 1);
}

#[test]
fn follower_serves_not_found_recorded_by_the_leader() {
    // Arrange
    let (upstream_a, gate) = FakeUpstream::gated([Step::NotFound]);
    let world = World::with_upstream(upstream_a);
    let upstream_b = FakeUpstream::new([]);
    let replica = world.replica(upstream_b.clone());
    let (blob, negative) = (path("crates/a.crate"), NegativePolicy::ARTIFACT);
    let mut leader = pin!(world.engine.artifact(&world.origin, &blob, None, &negative));
    let mut follower = pin!(replica.artifact(&world.origin, &blob, None, &negative));
    let _ = poll_once(leader.as_mut());
    let _ = poll_once(follower.as_mut());
    gate.open();

    // Act
    let (_, follower) = block_on(join(leader, follower));

    // Assert
    assert!(matches!(follower, Err(ProxyError::NotFound)));
    assert_eq!(upstream_b.call_count(), 0);
}

#[test]
fn follower_fetches_itself_when_the_negative_entry_expired_before_settling() {
    // Arrange
    let world = World::new([]);
    let upstream_b = FakeUpstream::new([Step::Found(b"hello", None)]);
    let replica = world.replica(upstream_b.clone());
    let (blob, negative) = (path("crates/a.crate"), NegativePolicy::ARTIFACT);
    let lease = match block_on(
        world
            .leases
            .claim(&key("crates/a.crate"), secs(30), secs(25)),
    )
    .unwrap()
    {
        Claim::Lead(lease) => lease,
        Claim::Settled => unreachable!("lease свободен"),
    };
    let mut follower = pin!(replica.artifact(&world.origin, &blob, None, &negative));
    let _ = poll_once(follower.as_mut());
    let recorded = Cached {
        fetched_at: common::proxy::epoch(),
        state: CachedState::Missing { misses: 1 },
    };
    block_on(world.cache.put(&key("crates/a.crate"), recorded, secs(600))).unwrap();
    world.advance(negative.ttl);
    block_on(world.leases.release(lease)).unwrap();

    // Act
    let served = block_on(follower).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(upstream_b.call_count(), 1);
}

#[test]
fn dropped_leader_does_not_block_the_next_request_for_the_key() {
    // Arrange
    let (upstream_a, gate) = FakeUpstream::gated([Step::Found(b"v1", None)]);
    let world = World::with_upstream(upstream_a);
    let replica = world.replica(FakeUpstream::new([Step::Found(b"v2", None)]));
    let (doc, policy) = (path(DOC), policy());
    let mut abandoned = Box::pin(world.engine.metadata(&world.origin, &doc, &policy));
    let _ = poll_once(abandoned.as_mut());
    drop(abandoned);
    gate.open();

    // Act
    let outcome = poll_once(pin!(replica.metadata(&world.origin, &doc, &policy)));

    // Assert
    let Poll::Ready(Ok(served)) = outcome else {
        panic!("запрос завис на lease брошенного лидера: {outcome:?}");
    };
    assert_eq!(served.result, CacheResult::Miss);
}
