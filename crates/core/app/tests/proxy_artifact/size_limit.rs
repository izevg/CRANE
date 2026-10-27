//! Потолок размера артефакта.

use crane_app::proxy::{CacheResult, ProxyError, ProxyLimits};
use crane_domain::registry::Sha256;
use crane_ports::proxy::ProxyCache;
use crane_ports::registry::{BlobError, BlobStore};
use futures_executor::block_on;

use crate::common::proxy::upstream::Step;
use crate::common::proxy::{World, bytes_of, key};

use super::CRATE;

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
