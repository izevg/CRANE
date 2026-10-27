//! CAS: промах кладёт в хранилище, повтор берёт оттуда, потерянный blob качается заново.

use std::sync::Arc;

use crane_app::proxy::{CacheResult, ProxyEngine, ProxyError, ProxyLimits};
use crane_domain::proxy::NegativePolicy;
use crane_memory::{ManualClock, MemoryBlobStore};
use crane_ports::proxy::{Cached, CachedState, ProxyCache};
use crane_ports::registry::{BlobError, Digests};
use futures_executor::block_on;

use crate::common::faults::OpenFailsBlobs;
use crate::common::proxy::ports::{SharedCache, SharedLeases};
use crate::common::proxy::upstream::{FakeUpstream, Step};
use crate::common::proxy::{SharedBlobs, World, bytes_of, epoch, key, origin, path, secs};
use crate::common::{hello_sha1, hello_sha256, hello_sha512};

use super::CRATE;

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
