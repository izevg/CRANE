//! Получение: скачивание файла и список версий; клиенты видят только `Released`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::{HOSTED, Seed, asset_path, coordinate, name, read};
use crane_app::registry::RegistryError;
use crane_domain::registry::{Format, VersionState};
use futures_executor::block_on;

#[test]
fn fetch_returns_released_asset_with_uploaded_bytes() {
    // Arrange
    let seed = Seed::new();
    let stored = seed.add_version("foo", "1.0.0", VersionState::Released, b"payload");
    let registry = seed.registry();

    // Act
    let download =
        block_on(registry.fetch(&coordinate("foo", "1.0.0"), &asset_path("foo", "1.0.0"))).unwrap();

    // Assert
    assert_eq!(
        download.asset, stored.assets[0],
        "asset совпадает с записью каталога"
    );
    assert_eq!(read(download.body), b"payload");
}

#[test]
fn fetch_serves_yanked_version() {
    // Arrange
    let seed = Seed::new();
    let stored = seed.add_version("foo", "1.0.0", VersionState::Released, b"payload");
    seed.yank(&stored);
    let registry = seed.registry();

    // Act
    let download =
        block_on(registry.fetch(&coordinate("foo", "1.0.0"), &asset_path("foo", "1.0.0"))).unwrap();

    // Assert
    assert_eq!(
        read(download.body),
        b"payload",
        "yanked не мешает скачиванию: lock-файлы должны собираться"
    );
}

#[test]
fn fetch_hides_deleted_blocked_and_quarantined_versions() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("del", "1.0.0", VersionState::Deleted, b"x");
    seed.add_version("blk", "1.0.0", VersionState::Blocked, b"x");
    seed.add_version("qrn", "1.0.0", VersionState::Quarantined, b"x");
    let registry = seed.registry();

    // Act
    let outcomes = ["del", "blk", "qrn"].map(|package| {
        block_on(registry.fetch(&coordinate(package, "1.0.0"), &asset_path(package, "1.0.0")))
    });

    // Assert
    let states: Vec<_> = outcomes
        .into_iter()
        .map(|outcome| match outcome.unwrap_err() {
            RegistryError::Unavailable { state } => state,
            other => panic!("ожидался Unavailable: {other:?}"),
        })
        .collect();
    assert_eq!(
        states,
        [
            VersionState::Deleted,
            VersionState::Blocked,
            VersionState::Quarantined
        ],
        "не-Released версия недоступна, состояние попадает в ошибку"
    );
}

#[test]
fn fetch_unknown_version_is_not_found() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let error =
        block_on(registry.fetch(&coordinate("foo", "1.0.0"), "foo-1.0.0.crate")).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::VersionNotFound), "{error:?}");
}

#[test]
fn fetch_from_unknown_repository_is_not_found() {
    // Arrange
    let registry = Seed::new().registry();
    let mut target = coordinate("foo", "1.0.0");
    target.repository = "nope".to_owned();

    // Act
    let error = block_on(registry.fetch(&target, "foo-1.0.0.crate")).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::RepositoryNotFound),
        "{error:?}"
    );
}

#[test]
fn fetch_of_unknown_path_is_asset_not_found() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"payload");
    let registry = seed.registry();

    // Act
    let error = block_on(registry.fetch(&coordinate("foo", "1.0.0"), "other.crate")).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::AssetNotFound), "{error:?}");
}

#[test]
fn fetch_with_record_but_missing_blob_is_backend_error() {
    // Arrange
    let seed = Seed::new();
    seed.add_dangling("foo", "1.0.0");
    let registry = seed.registry();

    // Act
    let error = block_on(registry.fetch(&coordinate("foo", "1.0.0"), &asset_path("foo", "1.0.0")))
        .unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::Backend(_)),
        "запись без blob — нарушенный инвариант (500), а не AssetNotFound: {error:?}"
    );
}

#[test]
fn versions_lists_only_released_in_publication_order_including_yanked() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"1");
    let yanked = seed.add_version("foo", "1.1.0", VersionState::Released, b"2");
    seed.add_version("foo", "1.2.0", VersionState::Deleted, b"3");
    seed.add_version("foo", "1.3.0", VersionState::Blocked, b"4");
    seed.add_version("foo", "1.4.0", VersionState::Quarantined, b"5");
    seed.add_version("foo", "2.0.0", VersionState::Released, b"6");
    seed.yank(&yanked);
    let registry = seed.registry();

    // Act
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("foo"))).unwrap();

    // Assert
    let shown: Vec<_> = listed
        .iter()
        .map(|v| (v.version.as_written().to_owned(), v.yanked))
        .collect();
    assert_eq!(
        shown,
        [
            ("1.0.0".to_owned(), false),
            ("1.1.0".to_owned(), true),
            ("2.0.0".to_owned(), false)
        ],
        "клиенты видят только Released, yanked остаётся в списке с флагом"
    );
}

#[test]
fn versions_of_unknown_package_is_empty() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let listed = block_on(registry.versions(Format::Cargo, HOSTED, &name("ghost"))).unwrap();

    // Assert
    assert!(
        listed.is_empty(),
        "неизвестный пакет — пустой список, не ошибка"
    );
}

#[test]
fn versions_of_unknown_repository_is_not_found() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let error = block_on(registry.versions(Format::Cargo, "nope", &name("foo"))).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::RepositoryNotFound),
        "{error:?}"
    );
}
