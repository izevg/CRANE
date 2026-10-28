#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use std::io;

use super::*;

fn sha(byte: u8) -> Sha256 {
    Sha256::from([byte; 32])
}

fn is_backend(error: &RegistryError) -> bool {
    matches!(error, RegistryError::Backend(_))
}

#[test]
fn catalog_version_exists_maps_to_version_exists() {
    // Arrange
    let error = CatalogError::VersionExists;

    // Act
    let mapped = RegistryError::from(error);

    // Assert
    assert!(
        matches!(mapped, RegistryError::VersionExists),
        "tombstone и дубликат должны давать 409, а не 500: {mapped:?}"
    );
}

#[test]
fn catalog_asset_exists_maps_to_asset_exists() {
    // Arrange
    let error = CatalogError::AssetExists;

    // Act
    let mapped = RegistryError::from(error);

    // Assert
    assert!(
        matches!(mapped, RegistryError::AssetExists),
        "занятый путь файла должен давать 409, а не 500: {mapped:?}"
    );
}

#[test]
fn catalog_broken_invariants_map_to_backend() {
    // Arrange
    let errors = [
        CatalogError::NotFound,
        CatalogError::StateConflict {
            actual: VersionState::Deleted,
        },
        CatalogError::Backend("сбой".into()),
    ];

    // Act
    let mapped = errors.map(RegistryError::from);

    // Assert
    assert!(
        mapped.iter().all(is_backend),
        "прочие ошибки каталога — 500: {mapped:?}"
    );
}

#[test]
fn blob_body_error_maps_to_body() {
    // Arrange
    let error = BlobError::Body(io::Error::other("обрыв"));

    // Act
    let mapped = RegistryError::from(error);

    // Assert
    assert!(
        matches!(mapped, RegistryError::Body(_)),
        "обрыв тела — ошибка клиента: {mapped:?}"
    );
}

#[test]
fn other_blob_errors_map_to_backend() {
    // Arrange
    let errors = [
        BlobError::NotFound(sha(1)),
        BlobError::Backend("сбой".into()),
    ];

    // Act
    let mapped = errors.map(RegistryError::from);

    // Assert
    assert!(
        mapped.iter().all(is_backend),
        "отсутствующий blob при записи каталога — 500, не AssetNotFound: {mapped:?}"
    );
}
