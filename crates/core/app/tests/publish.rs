//! Публикация: атомарность, неизменяемость версий, проверка заявленного sha256.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::io;

use bytes::Bytes;
use common::{
    HOSTED, InstrumentedCatalog, RecordingBlobs, Seed, assert_send, asset_path, commit_log,
    committed, coordinate, fixed_now, hello_sha256, name, publish_request, read,
};
use crane_app::registry::{Coordinate, Publish, RegistryError, Upload};
use crane_domain::registry::{Format, FormatMetadata, Sha256, VersionId, VersionState};
use crane_memory::SequentialIdGen;
use crane_ports::system::IdGen;
use futures_executor::block_on;
use futures_util::stream;

const HELLO: [&[u8]; 2] = [b"hel", b"lo"];

#[test]
fn publish_stores_blob_and_released_record() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, None);
    // Репозиторий в Seed взял первый id, версии достанется второй.
    let probe = SequentialIdGen::default();
    probe.generate();
    let expected_id = VersionId::new(probe.generate());

    // Act
    let record = block_on(registry.publish(request)).unwrap();

    // Assert
    assert_eq!(
        record.state,
        VersionState::Released,
        "hosted публикует сразу в Released"
    );
    assert!(!record.yanked, "новая версия не yanked");
    assert_eq!(record.published_at, fixed_now(), "время берётся из Clock");
    assert_eq!(record.id, expected_id, "id берётся из IdGen");
    assert_eq!(
        record.metadata.as_str(),
        common::METADATA,
        "метаданные сохраняются как есть"
    );
    assert_eq!(record.assets.len(), 1, "один файл на запрос");
    let asset = &record.assets[0];
    assert_eq!(asset.path, "foo-1.0.0.crate");
    assert_eq!(asset.sha256, hello_sha256(), "sha256 считает хранилище");
    assert_eq!(asset.size, 5, "размер равен числу байт");
    let download = block_on(registry.fetch(&coordinate("foo", "1.0.0"), &asset.path)).unwrap();
    assert_eq!(
        read(download.body),
        b"hello",
        "blob лежит в CAS с теми же байтами"
    );
}

#[test]
fn publish_accepts_matching_declared_sha256() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Some(hello_sha256()));

    // Act
    let result = block_on(registry.publish(request));

    // Assert
    assert!(
        result.is_ok(),
        "совпавший заявленный sha256 не мешает публикации: {result:?}"
    );
}

#[test]
fn publish_rejects_mismatching_declared_sha256_without_commit_or_record() {
    // Arrange
    let declared = Sha256::from([9; 32]);
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::new(blobs, log.clone(), false),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, Some(declared));

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(
            error,
            RegistryError::DigestMismatch { declared: d, actual } if d == declared && actual == hello_sha256()
        ),
        "ожидался DigestMismatch с точными значениями: {error:?}"
    );
    assert!(
        committed(&log).is_empty(),
        "неверный blob не должен попасть в CAS"
    );
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("foo"))).unwrap();
    assert!(listed.is_empty(), "записи каталога быть не должно");
}

#[test]
fn publish_to_unknown_repository_fails() {
    // Arrange
    let registry = Seed::new().registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, None);
    request.coordinate.repository = "nope".to_owned();

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::RepositoryNotFound),
        "{error:?}"
    );
}

#[test]
fn publish_to_proxy_repository_is_forbidden() {
    // Arrange
    let registry = Seed::new().with_proxy().registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, None);
    request.coordinate.repository = common::PROXY.to_owned();

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::NotHosted),
        "в proxy публиковать нельзя: {error:?}"
    );
}

#[test]
fn republish_with_other_bytes_keeps_original_and_commits_nothing() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"original");
    let log = commit_log();
    let registry = seed.registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::new(blobs, log.clone(), false),
    );
    let request = publish_request("foo", "1.0.0", &[b"different"], None);

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "версия неизменяема: {error:?}"
    );
    assert!(
        committed(&log).is_empty(),
        "предпроверка должна отказать до commit: нет blob-сироты"
    );
    let download =
        block_on(registry.fetch(&coordinate("foo", "1.0.0"), &asset_path("foo", "1.0.0"))).unwrap();
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
    seed.add_moved("foo", "1.0.0", VersionState::Deleted, b"gone");
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "номер удалённой версии занят навсегда: {error:?}"
    );
}

#[test]
fn republish_with_other_spelling_of_same_canonical_key_is_rejected() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("Foo-Bar", "1.0.0", VersionState::Released, b"first");
    let registry = seed.registry();
    let request = publish_request("foo_bar", "1.0.0+build.1", &HELLO, None);

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "ключ сравнивается по канонической форме: {error:?}"
    );
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("foo-bar"))).unwrap();
    assert_eq!(listed.len(), 1, "вторая запись не появилась");
    assert_eq!(
        listed[0].package.as_written(),
        "Foo-Bar",
        "сохранено первое написание"
    );
}

#[test]
fn publish_lost_race_maps_catalog_conflict_to_version_exists() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"winner");
    let registry = seed.registry_with(InstrumentedCatalog::blind, |blobs| blobs);
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "конфликт на publish_version — 409, а не 500: {error:?}"
    );
}

#[test]
fn publish_with_broken_body_reports_body_error_and_publishes_nothing() {
    // Arrange
    let registry = Seed::new().registry();
    let items = vec![
        Ok(Bytes::from_static(b"ab")),
        Err(io::Error::other("обрыв")),
    ];
    let request = Publish {
        coordinate: coordinate("foo", "1.0.0"),
        file: Upload {
            path: asset_path("foo", "1.0.0"),
            body: Box::pin(stream::iter(items)),
            declared_sha256: None,
        },
        metadata: FormatMetadata::new(common::METADATA.to_owned()),
    };

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::Body(_)),
        "обрыв тела — ошибка клиента: {error:?}"
    );
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("foo"))).unwrap();
    assert!(listed.is_empty(), "версия не опубликована");
}

#[test]
fn publish_never_records_version_when_blob_commit_fails() {
    // Arrange
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::new(blobs, log.clone(), true),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::Backend(_)), "{error:?}");
    assert_eq!(
        committed(&log),
        vec![hello_sha256()],
        "commit был предпринят"
    );
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("foo"))).unwrap();
    assert!(
        listed.is_empty(),
        "запись каталога не должна ссылаться на незакоммиченный blob"
    );
}

#[test]
fn use_case_futures_are_send() {
    // Arrange
    let registry = Seed::new().registry();
    let target: Coordinate = coordinate("foo", "1.0.0");
    let package = name("foo");

    // Act
    let publish = registry.publish(publish_request("foo", "1.0.0", &HELLO, None));
    let fetch = registry.fetch(&target, "foo-1.0.0.crate");
    let versions = registry.versions(Format::Cargo, HOSTED, &package);
    let yank = registry.set_yanked(&target, true);

    // Assert
    assert_send(&publish);
    assert_send(&fetch);
    assert_send(&versions);
    assert_send(&yank);
}
