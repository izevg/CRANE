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
    BlindCatalog, HOSTED, RecordingBlobs, Seed, asset_path, commit_log, committed, coordinate,
    fixed_now, hello_sha256, name, publish_request, read,
};
use crane_app::registry::{Publish, RegistryError, Upload};
use crane_domain::registry::{Format, FormatMetadata, Sha256, VersionState};
use futures_executor::block_on;
use futures_util::stream;

const HELLO: [&[u8]; 2] = [b"hel", b"lo"];

#[test]
fn publish_creates_released_not_yanked_record_stamped_by_clock() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, None);

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
}

#[test]
fn publish_keeps_metadata_as_is() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let record = block_on(registry.publish(request)).unwrap();

    // Assert
    assert_eq!(record.metadata.as_str(), common::METADATA);
}

#[test]
fn publish_records_single_asset_with_digest_and_size_computed_by_store() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let record = block_on(registry.publish(request)).unwrap();

    // Assert
    let assets: Vec<_> = record
        .assets
        .iter()
        .map(|asset| (asset.path.as_str(), asset.sha256, asset.size))
        .collect();
    assert_eq!(assets, [("foo-1.0.0.crate", hello_sha256(), 5)]);
}

#[test]
fn published_file_is_fetchable_with_uploaded_bytes() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    block_on(registry.publish(request)).unwrap();

    // Assert
    let download =
        block_on(registry.fetch(&coordinate("foo", "1.0.0"), &asset_path("foo", "1.0.0"))).unwrap();
    assert_eq!(
        read(download.body),
        b"hello",
        "blob лежит в CAS с теми же байтами"
    );
}

#[test]
fn publish_takes_version_id_from_shared_generator() {
    // Arrange
    // Seed::new() взял из генератора 1 для hosted-репозитория; хранилище blob генератор
    // не трогает, так что первый id версии — 2.
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let record = block_on(registry.publish(request)).unwrap();

    // Assert
    assert_eq!(record.id.as_uuid().as_u128(), 2);
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
fn publish_rejects_mismatching_declared_sha256() {
    // Arrange
    let declared = Sha256::from([9; 32]);
    let registry = Seed::new().registry();
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
}

#[test]
fn digest_mismatch_commits_nothing_to_cas() {
    // Arrange
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, Some(Sha256::from([9; 32])));

    // Act
    let _rejected = block_on(registry.publish(request));

    // Assert
    assert!(
        committed(&log).is_empty(),
        "неверный blob не должен попасть в CAS"
    );
}

#[test]
fn digest_mismatch_leaves_no_catalog_record() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Some(Sha256::from([9; 32])));

    // Act
    let _rejected = block_on(registry.publish(request));

    // Assert
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
fn republish_with_other_bytes_is_rejected_as_version_exists() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"original");
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &[b"different"], None);

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "версия неизменяема: {error:?}"
    );
}

#[test]
fn rejected_republish_commits_nothing_to_cas() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"original");
    let log = commit_log();
    let registry = seed.registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &[b"different"], None);

    // Act
    let _rejected = block_on(registry.publish(request));

    // Assert
    assert!(
        committed(&log).is_empty(),
        "предпроверка должна отказать до commit: нет blob-сироты"
    );
}

#[test]
fn rejected_republish_keeps_original_bytes() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"original");
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &[b"different"], None);

    // Act
    let _rejected = block_on(registry.publish(request));

    // Assert
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
    seed.add_version("foo", "1.0.0", VersionState::Deleted, b"gone");
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
}

#[test]
fn rejected_other_spelling_keeps_only_first_record() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("Foo-Bar", "1.0.0", VersionState::Released, b"first");
    let registry = seed.registry();
    let request = publish_request("foo_bar", "1.0.0+build.1", &HELLO, None);

    // Act
    let _rejected = block_on(registry.publish(request));

    // Assert
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("foo-bar"))).unwrap();
    let spellings: Vec<_> = listed.iter().map(|v| v.package.as_written()).collect();
    assert_eq!(spellings, ["Foo-Bar"], "сохранено только первое написание");
}

#[test]
fn publish_lost_race_maps_catalog_conflict_to_version_exists() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"winner");
    let registry = seed.registry_with(BlindCatalog, |blobs| blobs);
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "конфликт на publish_version — 409, а не 500: {error:?}"
    );
}

/// Запрос, тело которого обрывается ошибкой ввода-вывода.
fn broken_body_request() -> Publish {
    let items = vec![
        Ok(Bytes::from_static(b"ab")),
        Err(io::Error::other("обрыв")),
    ];
    Publish {
        coordinate: coordinate("foo", "1.0.0"),
        file: Upload {
            path: asset_path("foo", "1.0.0"),
            body: Box::pin(stream::iter(items)),
            declared_sha256: None,
        },
        metadata: FormatMetadata::new(common::METADATA.to_owned()),
    }
}

#[test]
fn publish_with_broken_body_reports_body_error() {
    // Arrange
    let registry = Seed::new().registry();
    let request = broken_body_request();

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::Body(_)),
        "обрыв тела — ошибка клиента: {error:?}"
    );
}

#[test]
fn publish_with_broken_body_publishes_nothing() {
    // Arrange
    let registry = Seed::new().registry();
    let request = broken_body_request();

    // Act
    let _rejected = block_on(registry.publish(request));

    // Assert
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("foo"))).unwrap();
    assert!(listed.is_empty(), "версия не опубликована");
}

#[test]
fn publish_reports_backend_error_when_blob_commit_fails() {
    // Arrange
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::failing_commit(blobs, commit_log()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let error = block_on(registry.publish(request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::Backend(_)), "{error:?}");
}

#[test]
fn publish_never_records_version_when_blob_commit_fails() {
    // Arrange
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::failing_commit(blobs, commit_log()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, None);

    // Act
    let _rejected = block_on(registry.publish(request));

    // Assert
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("foo"))).unwrap();
    assert!(
        listed.is_empty(),
        "запись каталога не должна ссылаться на незакоммиченный blob"
    );
}
