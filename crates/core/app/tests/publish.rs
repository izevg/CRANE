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
    BlindCatalog, HOSTED, RecordingBlobs, Seed, asset_path, caller, commit_log, committed,
    coordinate, fixed_now, hello_sha1, hello_sha256, hello_sha512, name, publish_request, read,
    version,
};
use crane_app::registry::{Publish, Registry, RegistryError, Upload};
use crane_domain::registry::{
    Asset, Checksum, Format, FormatMetadata, PackageVersion, Sha1, Sha256, VersionState,
};
use crane_ports::registry::{BlobStore, Catalog};
use futures_executor::block_on;
use futures_util::stream;

const HELLO: [&[u8]; 2] = [b"hel", b"lo"];

#[test]
fn publish_creates_released_record_with_store_digests_and_clock_time() {
    // Arrange
    let seed = Seed::new();
    let hosted = seed.hosted;
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let record = block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    let expected = PackageVersion {
        // id проверяет отдельный тест: здесь он не часть правила.
        id: record.id,
        repository: hosted,
        package: name("foo"),
        version: version("1.0.0"),
        state: VersionState::Released,
        yanked: false,
        deprecated: None,
        published_at: fixed_now(),
        assets: vec![Asset {
            path: asset_path("foo", "1.0.0"),
            sha256: hello_sha256(),
            size: 5,
        }],
        metadata: FormatMetadata::new(common::METADATA.to_owned()),
    };
    assert_eq!(record, expected);
}

#[test]
fn published_file_is_fetchable_with_uploaded_bytes() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    let download = block_on(registry.fetch(
        &caller(),
        &coordinate("foo", "1.0.0"),
        &asset_path("foo", "1.0.0"),
    ))
    .unwrap();
    assert_eq!(
        read(download.body),
        b"hello",
        "blob лежит в CAS с теми же байтами"
    );
}

#[test]
fn publish_takes_version_id_from_shared_generator() {
    // Arrange
    // Генератор общий с фикстурой: id версии — следующий после id hosted-репозитория.
    let seed = Seed::new();
    let hosted = seed.hosted;
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let record = block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    assert_eq!(
        record.id.as_uuid().as_u128(),
        hosted.as_uuid().as_u128() + 1
    );
}

#[test]
fn publish_accepts_matching_declared_sha256() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request(
        "foo",
        "1.0.0",
        &HELLO,
        vec![Checksum::Sha256(hello_sha256())],
    );

    // Act
    let result = block_on(registry.publish(&caller(), request));

    // Assert
    assert!(
        result.is_ok(),
        "совпавший заявленный sha256 не мешает публикации: {result:?}"
    );
}

#[test]
fn publish_rejects_mismatching_declared_sha256_leaving_cas_and_catalog_untouched() {
    // Arrange
    let declared = Checksum::Sha256(Sha256::from([9; 32]));
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, vec![declared]);

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(
            &error,
            RegistryError::DigestMismatch { declared: d, actual } if **d == declared && **actual == Checksum::Sha256(hello_sha256())
        ),
        "ожидался DigestMismatch с точными значениями: {error:?}"
    );
    assert!(
        committed(&log).is_empty(),
        "неверный blob не должен попасть в CAS"
    );
    assert!(
        listed_foo(&registry).is_empty(),
        "записи каталога быть не должно"
    );
}

#[test]
fn publish_to_unknown_repository_fails() {
    // Arrange
    let registry = Seed::new().registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.coordinate.repository = "nope".to_owned();

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

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
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.coordinate.repository = common::PROXY.to_owned();

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::NotHosted),
        "в proxy публиковать нельзя: {error:?}"
    );
}

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
    let registry = seed.registry_with(BlindCatalog, |blobs| blobs);
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::VersionExists),
        "конфликт на publish_version — 409, а не 500: {error:?}"
    );
}

/// Наблюдение для Assert: версии пакета `foo`, видимые клиентам.
fn listed_foo<C: Catalog, B: BlobStore>(registry: &Registry<C, B>) -> Vec<PackageVersion> {
    block_on(registry.versions(&caller(), Format::Cargo, HOSTED, &name("foo"))).unwrap()
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
            declared: Vec::new(),
        },
        metadata: FormatMetadata::new(common::METADATA.to_owned()),
        tags: Vec::new(),
    }
}

#[test]
fn publish_with_broken_body_reports_body_error_and_publishes_nothing() {
    // Arrange
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = broken_body_request();

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::Body(_)),
        "обрыв тела — ошибка клиента: {error:?}"
    );
    assert!(
        committed(&log).is_empty(),
        "оборванное тело не попадает в CAS"
    );
    assert!(listed_foo(&registry).is_empty(), "версия не опубликована");
}

#[test]
fn publish_reports_backend_error_and_records_nothing_when_blob_commit_fails() {
    // Arrange
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::failing_commit(blobs, commit_log()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::Backend(_)), "{error:?}");
    assert!(
        listed_foo(&registry).is_empty(),
        "запись каталога не должна ссылаться на незакоммиченный blob"
    );
}

#[test]
fn publish_rejects_when_any_declared_checksum_mismatches_leaving_cas_untouched() {
    // Arrange
    let declared = [
        Checksum::Sha256(hello_sha256()),
        Checksum::Sha1(Sha1::from([9; 20])),
    ];
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, declared.to_vec());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(&error, RegistryError::DigestMismatch { declared: d, actual }
            if **d == declared[1] && **actual == Checksum::Sha1(hello_sha1())),
        "{error:?}"
    );
    assert!(committed(&log).is_empty(), "неверный blob не попал в CAS");
}

#[test]
fn publish_accepts_all_matching_declared_checksums() {
    // Arrange
    let registry = Seed::new().registry();
    let declared = vec![
        Checksum::Sha256(hello_sha256()),
        Checksum::Sha512(hello_sha512()),
        Checksum::Sha1(hello_sha1()),
    ];
    let request = publish_request("foo", "1.0.0", &HELLO, declared);

    // Act
    let result = block_on(registry.publish(&caller(), request));

    // Assert
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn publish_with_tags_sets_them_on_the_new_version() {
    // Arrange
    let registry = Seed::new().registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.tags = vec!["latest".to_owned()];

    // Act
    let record = block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    let package =
        block_on(registry.package(&caller(), Format::Cargo, HOSTED, &name("foo"))).unwrap();
    assert_eq!(package.tags, [("latest".to_owned(), record.id)]);
}
