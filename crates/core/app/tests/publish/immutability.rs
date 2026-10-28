//! Неизменяемость версий: повторная публикация и гонка.

use crane_app::registry::RegistryError;
use crane_domain::registry::{Format, VersionState};
use futures_executor::block_on;

use crate::common::faults::{BlindCatalog, RecordingBlobs, commit_log, committed};
use crate::common::{HOSTED, Seed, asset_path, caller, coordinate, name, publish_request, read};

use super::HELLO;

#[test]
fn republish_with_other_bytes_is_rejected_leaving_original_untouched() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"original");
    let log = commit_log();
    let registry = seed.registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &[b"different"], Vec::new());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "версия неизменяема: {error:?}"
    );
    assert!(
        committed(&log).is_empty(),
        "предпроверка должна отказать до commit: нет blob-сироты"
    );
    let download = block_on(registry.fetch(
        &caller(),
        &coordinate("foo", "1.0.0"),
        &asset_path("foo", "1.0.0"),
    ))
    .unwrap();
    assert_eq!(
        read(download.body),
        b"original",
        "исходные байты не изменились"
    );
}

#[test]
fn republish_over_tombstone_is_rejected() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Deleted, b"gone");
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "номер удалённой версии занят навсегда: {error:?}"
    );
}

#[test]
fn republish_with_other_spelling_of_same_canonical_key_is_rejected_keeping_first() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("Foo-Bar", "1.0.0", VersionState::Released, b"first");
    let log = commit_log();
    let registry = seed.registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo_bar", "1.0.0+build.1", &HELLO, Vec::new());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "ключ сравнивается по канонической форме: {error:?}"
    );
    assert!(committed(&log).is_empty(), "blob-сироты в CAS нет");
    let listed =
        block_on(registry.versions(&caller(), Format::Cargo, HOSTED, &name("foo-bar"))).unwrap();
    let spellings: Vec<_> = listed.iter().map(|v| v.package.as_written()).collect();
    assert_eq!(spellings, ["Foo-Bar"], "сохранено только первое написание");
}

#[test]
fn publish_lost_race_maps_catalog_conflict_to_version_exists() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"winner");
    let registry = seed.registry_with(BlindCatalog::always, |blobs| blobs);
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "конфликт на publish_version — 409, а не 500: {error:?}"
    );
}
