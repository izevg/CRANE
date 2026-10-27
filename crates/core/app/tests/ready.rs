//! Готовность: оба хранилища отвечают на безобидный вопрос.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::Seed;
use common::faults::{DownCatalog, OpenFailsBlobs};
use crane_app::registry::RegistryError;
use crane_domain::registry::{Format, Sha256};
use crane_ports::registry::BlobError;
use futures_executor::block_on;

#[test]
fn registry_over_healthy_stores_is_ready() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let result = block_on(registry.ready());

    // Assert
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn unreachable_catalog_makes_registry_not_ready() {
    // Arrange
    let registry = Seed::new().registry_with(DownCatalog, |blobs| blobs);

    // Act
    let result = block_on(registry.ready());

    // Assert
    assert!(
        matches!(result, Err(RegistryError::Backend(_))),
        "{result:?}"
    );
}

#[test]
fn blob_backend_failure_makes_registry_not_ready() {
    // Arrange
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| OpenFailsBlobs::new(blobs, || BlobError::Backend("диск недоступен".into())),
    );

    // Act
    let result = block_on(registry.ready());

    // Assert
    assert!(
        matches!(result, Err(RegistryError::Backend(_))),
        "{result:?}"
    );
}

#[test]
fn missing_probe_blob_still_means_blob_store_answered() {
    // Arrange
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| OpenFailsBlobs::new(blobs, || BlobError::NotFound(Sha256::from([0; 32]))),
    );

    // Act
    let result = block_on(registry.ready());

    // Assert
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn unknown_repository_is_not_found_and_known_one_is_returned() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let results = ["nope", common::HOSTED].map(|name| {
        block_on(registry.repository(&common::caller(), Format::Cargo, name))
            .map(|repository| repository.name)
            .map_err(|error| matches!(error, RegistryError::RepositoryNotFound))
    });

    // Assert
    assert_eq!(results, [Err(true), Ok(common::HOSTED.to_owned())]);
}
