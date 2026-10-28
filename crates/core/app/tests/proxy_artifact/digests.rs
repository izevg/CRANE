//! Проверка контрольных сумм артефакта: ожидаемый дайджест, чужой blob, sha512.

use crane_app::proxy::{CacheResult, ProxyError};
use crane_domain::registry::{Checksum, Sha256, Sha512};
use crane_ports::proxy::ProxyCache;
use crane_ports::registry::{BlobError, BlobStore};
use futures_executor::block_on;

use crate::common::proxy::upstream::Step;
use crate::common::proxy::{World, bytes_of, key};
use crate::common::{hello_sha256, hello_sha512};

use super::CRATE;

const WORLD_SHA256: &str = "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7";

const WORLD_SHA512: &str = "11853df40f4b2b919d3815f64792e58d08663767a494bcbb38c0b2389d9140bbb170281b4a847be7757bde12c9cd0054ce3652d0ad3a1a0c92babb69798246ee";

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
        Err(ProxyError::DigestMismatch { expected: e, actual }) if *e == expected && *actual == Checksum::Sha256(hello_sha256())
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
        Err(ProxyError::DigestMismatch { expected: e, actual }) if *e == expected && *actual == Checksum::Sha256(hello_sha256())
    ));
    assert_eq!(world.upstream.call_count(), 2);
}

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
            if *e == expected && *actual == Checksum::Sha512(hello_sha512())
    ));
    assert!(matches!(
        block_on(world.blobs.open(&hello_sha256())),
        Err(BlobError::NotFound(_))
    ));
}
