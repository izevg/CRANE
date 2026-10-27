//! Proxy-движок, артефакты: CAS, проверка sha256, перекачка потерянного blob, negative-кэш.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::sync::Arc;

use common::proxy::{
    FakeUpstream, SharedBlobs, SharedCache, SharedLeases, Step, World, bytes_of, epoch,
    just_before, key, origin, path, secs,
};
use common::{OpenFailsBlobs, hello_sha1, hello_sha256, hello_sha512};
use crane_app::proxy::{CacheResult, ProxyEngine, ProxyError, ProxyLimits, UpstreamFailure};
use crane_domain::proxy::{EgressRefusal, NegativePolicy};
use crane_domain::registry::{Checksum, Sha256, Sha512};
use crane_memory::{ManualClock, MemoryBlobStore};
use crane_ports::proxy::{Cached, CachedState, ProxyCache};
use crane_ports::registry::{BlobError, BlobStore, Digests};
use futures_executor::block_on;

const CRATE: &str = "crates/serde/serde-1.0.0.crate";

#[test]
fn miss_stores_in_cas_and_serves_upstream_bytes() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);

    // Act
    let served = world.artifact(CRATE, None).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(served.body.digests.sha256, hello_sha256());
    assert_eq!(served.body.digests.size, 5);
    assert_eq!(bytes_of(served.body), b"hello");
}

#[test]
fn repeat_is_hit_from_cas_without_upstream_call() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);
    world.artifact(CRATE, None).unwrap();
    world.advance(secs(3600));

    // Act
    let served = world.artifact(CRATE, None).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert_eq!(served.age, secs(3600));
    assert_eq!(bytes_of(served.body), b"hello");
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn matching_expected_digest_is_accepted() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);

    // Act
    let served = world
        .artifact(CRATE, Some(Checksum::Sha256(hello_sha256())))
        .unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
}

#[test]
fn digest_mismatch_is_rejected() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);
    let expected = Checksum::Sha256(Sha256::from([0xab; 32]));

    // Act
    let outcome = world.artifact(CRATE, Some(expected));

    // Assert
    assert!(matches!(
        outcome,
        Err(ProxyError::DigestMismatch { expected: e, actual }) if e == expected && actual == Checksum::Sha256(hello_sha256())
    ));
}

#[test]
fn digest_mismatch_commits_nothing_to_cas() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);
    let _ = world.artifact(CRATE, Some(Checksum::Sha256(Sha256::from([0xab; 32]))));

    // Act
    let opened = block_on(world.blobs.open(&hello_sha256()));

    // Assert
    assert!(matches!(opened, Err(BlobError::NotFound(_))));
}

#[test]
fn digest_mismatch_leaves_no_cache_entry() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);
    let _ = world.artifact(CRATE, Some(Checksum::Sha256(Sha256::from([0xab; 32]))));

    // Act
    let entry = block_on(world.cache.get(&key(CRATE))).unwrap();

    // Assert
    assert_eq!(entry, None);
}

#[test]
fn lost_blob_is_downloaded_again_as_miss() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);
    world.artifact(CRATE, None).unwrap();
    let upstream = FakeUpstream::new([Step::Found(b"hello", None)]);
    let engine = world.replica_with_blobs(upstream.clone(), SharedBlobs::default());

    // Act
    let served =
        block_on(engine.artifact(&world.origin, &path(CRATE), None, &NegativePolicy::ARTIFACT))
            .unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(bytes_of(served.body), b"hello");
    assert_eq!(upstream.call_count(), 1);
}

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

#[test]
fn cas_backend_failure_on_hit_is_a_backend_error() {
    // Arrange
    let cache = SharedCache::default();
    let record = Cached {
        fetched_at: epoch(),
        state: CachedState::Blob(Digests {
            sha256: hello_sha256(),
            sha512: hello_sha512(),
            sha1: hello_sha1(),
            size: 5,
        }),
    };
    block_on(cache.put(&key(CRATE), record, secs(1))).unwrap();
    let clock = Arc::new(ManualClock::new(epoch()));
    let engine = ProxyEngine::new(
        FakeUpstream::new([]),
        cache,
        SharedLeases::new(Arc::clone(&clock)),
        OpenFailsBlobs::new(MemoryBlobStore::default(), || {
            BlobError::Backend("диск".into())
        }),
        clock,
        ProxyLimits::DEFAULT,
    );

    // Act
    let outcome =
        block_on(engine.artifact(&origin(), &path(CRATE), None, &NegativePolicy::ARTIFACT));

    // Assert
    assert!(matches!(outcome, Err(ProxyError::Backend(_))));
}

const WORLD_SHA256: &str = "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7";

#[test]
fn cached_blob_with_other_digest_is_not_served_and_fresh_bytes_win() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None), Step::Found(b"world", None)]);
    world.artifact(CRATE, None).unwrap();
    let expected = Checksum::Sha256(Sha256::from_hex(WORLD_SHA256).unwrap());

    // Act
    let served = world.artifact(CRATE, Some(expected)).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(Checksum::Sha256(served.body.digests.sha256), expected);
    assert_eq!(bytes_of(served.body), b"world");
    assert_eq!(world.upstream.call_count(), 2);
}

#[test]
fn cached_blob_with_other_digest_is_refetched_and_mismatch_reported() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None), Step::Found(b"hello", None)]);
    world.artifact(CRATE, None).unwrap();
    let expected = Checksum::Sha256(Sha256::from_hex(WORLD_SHA256).unwrap());

    // Act
    let outcome = world.artifact(CRATE, Some(expected));

    // Assert
    assert!(matches!(
        outcome,
        Err(ProxyError::DigestMismatch { expected: e, actual }) if e == expected && actual == Checksum::Sha256(hello_sha256())
    ));
    assert_eq!(world.upstream.call_count(), 2);
}

const WORLD_SHA512: &str = "11853df40f4b2b919d3815f64792e58d08663767a494bcbb38c0b2389d9140bbb170281b4a847be7757bde12c9cd0054ce3652d0ad3a1a0c92babb69798246ee";

#[test]
fn cached_blob_with_matching_sha512_is_served_without_upstream_call() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);
    world.artifact(CRATE, None).unwrap();

    // Act
    let served = world
        .artifact(CRATE, Some(Checksum::Sha512(hello_sha512())))
        .unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Hit);
    assert_eq!(world.upstream.call_count(), 1);
}

#[test]
fn cached_blob_with_other_sha512_is_refetched_and_fresh_bytes_win() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None), Step::Found(b"world", None)]);
    world.artifact(CRATE, None).unwrap();
    let expected = Checksum::Sha512(Sha512::from_hex(WORLD_SHA512).unwrap());

    // Act
    let served = world.artifact(CRATE, Some(expected)).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(world.upstream.call_count(), 2);
}

#[test]
fn miss_with_wrong_sha512_reports_mismatch_and_commits_nothing() {
    // Arrange
    let world = World::new([Step::Found(b"hello", None)]);
    let expected = Checksum::Sha512(Sha512::from([0xab; 64]));

    // Act
    let outcome = world.artifact(CRATE, Some(expected));

    // Assert
    assert!(matches!(
        outcome,
        Err(ProxyError::DigestMismatch { expected: e, actual })
            if e == expected && actual == Checksum::Sha512(hello_sha512())
    ));
    assert!(matches!(
        block_on(world.blobs.open(&hello_sha256())),
        Err(BlobError::NotFound(_))
    ));
}

const SIX_BYTES_SHA256: &str = "ce06092fb948d9ffac7d1a376e404b26b7575bcc11ee05a4615fef4fec3a308b";

fn artifact_limit(max_artifact_bytes: u64) -> ProxyLimits {
    ProxyLimits {
        max_artifact_bytes,
        ..ProxyLimits::DEFAULT
    }
}

#[test]
fn artifact_exactly_at_limit_is_served() {
    // Arrange
    let world = World::with_limits([Step::FoundChunks(&[b"he", b"llo"])], artifact_limit(5));

    // Act
    let served = world.artifact(CRATE, None).unwrap();

    // Assert
    assert_eq!(served.result, CacheResult::Miss);
    assert_eq!(bytes_of(served.body), b"hello");
}

#[test]
fn artifact_over_limit_is_rejected_and_leaves_no_blob_nor_cache_entry() {
    // Arrange
    let world = World::with_limits(
        [Step::FoundChunks(&[b"he", b"ll", b"o!"])],
        artifact_limit(5),
    );
    let oversized = Sha256::from_hex(SIX_BYTES_SHA256).unwrap();

    // Act
    let outcome = world.artifact(CRATE, None);

    // Assert
    assert!(matches!(outcome, Err(ProxyError::TooLarge { limit: 5 })));
    assert!(matches!(
        block_on(world.blobs.open(&oversized)),
        Err(BlobError::NotFound(_))
    ));
    assert_eq!(block_on(world.cache.get(&key(CRATE))).unwrap(), None);
}

#[test]
fn artifact_after_rejected_oversize_goes_to_upstream_again() {
    // Arrange
    let world = World::with_limits(
        [
            Step::FoundChunks(&[b"he", b"ll", b"o!"]),
            Step::Found(b"hello", None),
        ],
        artifact_limit(5),
    );
    let _ = world.artifact(CRATE, None);

    // Act
    let outcome = world.artifact(CRATE, None);

    // Assert
    assert_eq!(outcome.unwrap().result, CacheResult::Miss);
    assert_eq!(world.upstream.call_count(), 2);
}
