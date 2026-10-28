//! Занятый путь, повтор того же файла и сбои хранилищ при прикреплении.

use crane_app::registry::RegistryError;
use crane_domain::registry::{Checksum, Sha256, VersionState};
use futures_executor::block_on;

use crate::common::faults::{RecordingBlobs, commit_log, committed};
use crate::common::namespace::{Fault, FaultyCatalog};
use crate::common::{Seed, attach_request, broken_body, caller, hello_sha256};

use super::{FIRST, HELLO, SECOND, paths_of_foo};

/// Реестр с `foo 1.0.0`, в котором уже лежит файл `FIRST` с содержимым `hello`.
fn seeded() -> Seed {
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"hello");
    seed
}

#[test]
fn occupied_path_with_other_declared_sha256_is_asset_exists_without_reading_the_body() {
    // Arrange
    let registry = seeded().registry();
    let other = Checksum::Sha256(Sha256::from([1; 32]));
    let mut request = attach_request("foo", "1.0.0", FIRST, &HELLO, vec![other]);
    request.file.body = broken_body();

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::AssetExists),
        "тело читать нельзя, иначе вернулась бы ошибка тела: {error:?}"
    );
    assert_eq!(paths_of_foo(&registry), [FIRST]);
}

#[test]
fn occupied_path_with_other_content_is_asset_exists_and_keeps_the_stored_file() {
    // Arrange
    let registry = seeded().registry();
    let request = attach_request("foo", "1.0.0", FIRST, &[b"other"], Vec::new());

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    let stored =
        block_on(registry.fetch(&caller(), &crate::common::coordinate("foo", "1.0.0"), FIRST))
            .unwrap();
    assert!(matches!(error, RegistryError::AssetExists), "{error:?}");
    assert_eq!(crate::common::read(stored.body), b"hello");
}

#[test]
fn repeat_of_same_file_with_declared_sha256_succeeds_without_reading_the_body() {
    // Arrange
    let registry = seeded().registry();
    let declared = vec![Checksum::Sha256(hello_sha256())];
    let mut request = attach_request("foo", "1.0.0", FIRST, &HELLO, declared);
    request.file.body = broken_body();

    // Act
    let result = block_on(registry.attach(&caller(), request));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(paths_of_foo(&registry), [FIRST]);
}

#[test]
fn repeat_of_same_file_without_declared_sha256_succeeds_and_adds_nothing() {
    // Arrange
    let registry = seeded().registry();
    let request = attach_request("foo", "1.0.0", FIRST, &HELLO, Vec::new());

    // Act
    let result = block_on(registry.attach(&caller(), request));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(paths_of_foo(&registry), [FIRST]);
}

#[test]
fn broken_body_of_new_file_reports_body_error_and_keeps_the_version() {
    // Arrange
    let log = commit_log();
    let registry = seeded().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let mut request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());
    request.file.body = broken_body();

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::Body(_)), "{error:?}");
    assert!(committed(&log).is_empty(), "оборванное тело не в CAS");
    assert_eq!(paths_of_foo(&registry), [FIRST]);
}

#[test]
fn wrong_declared_digest_is_refused_before_commit() {
    // Arrange
    let log = commit_log();
    let registry = seeded().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let wrong = Checksum::Sha256(Sha256::from([1; 32]));
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, vec![wrong]);

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::DigestMismatch { .. }),
        "{error:?}"
    );
    assert!(committed(&log).is_empty(), "неверное содержимое не в CAS");
}

#[test]
fn catalog_failure_leaves_the_version_unchanged_and_the_blob_for_gc() {
    // Arrange
    let log = commit_log();
    let registry = seeded().registry_with(
        |inner| FaultyCatalog {
            inner,
            failing: &[Fault::AddAsset],
        },
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::Backend(_)), "{error:?}");
    assert_eq!(
        committed(&log).len(),
        1,
        "blob остаётся в CAS: сироту соберёт GC, как и при сбое публикации"
    );
    assert_eq!(paths_of_foo(&registry), [FIRST]);
}
