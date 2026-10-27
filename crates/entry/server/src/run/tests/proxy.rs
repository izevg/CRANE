#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use std::future::Future;

use crane_memory::{MemoryBlobStore, MemoryCatalog, SequentialIdGen};
use tokio::runtime::Builder;

use super::*;

use crate::config::{Cache, Repos};

const NAME: &str = "crates-io";

async fn seed(catalog: &MemoryCatalog, kind: RepositoryKind) {
    seed_as(catalog, Format::Cargo, kind).await;
}

async fn seed_as(catalog: &MemoryCatalog, format: Format, kind: RepositoryKind) {
    let ids = SequentialIdGen::default();
    catalog
        .ensure_repository(Repository {
            id: RepositoryId::new(ids.generate()),
            format,
            kind,
            name: NAME.to_owned(),
        })
        .await
        .unwrap();
}

async fn build(catalog: &MemoryCatalog) -> Result<Option<Prepared<MemoryBlobStore>>, RunError> {
    prepare_proxies(
        catalog,
        &SequentialIdGen::default(),
        Arc::new(MemoryBlobStore::default()),
        &Repos::of(Format::Cargo, RepositoryKind::Proxy, vec![NAME.to_owned()]),
        Proxy {
            cache: Cache::Memory,
            max_artifact_bytes: ProxyLimits::DEFAULT.max_artifact_bytes,
        },
    )
    .await
}

async fn build_npm(catalog: &MemoryCatalog) -> Result<Option<Prepared<MemoryBlobStore>>, RunError> {
    prepare_proxies(
        catalog,
        &SequentialIdGen::default(),
        Arc::new(MemoryBlobStore::default()),
        &Repos::of(Format::Npm, RepositoryKind::Proxy, vec![NAME.to_owned()]),
        Proxy {
            cache: Cache::Memory,
            max_artifact_bytes: ProxyLimits::DEFAULT.max_artifact_bytes,
        },
    )
    .await
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn existing_hosted_repository_blocks_startup_with_clear_error() {
    // Arrange
    let catalog = MemoryCatalog::default();
    block_on(seed(&catalog, RepositoryKind::Hosted));

    // Act
    let result = block_on(build(&catalog));

    // Assert
    let message = result.err().map(|error| error.to_string()).unwrap();
    assert!(message.contains("`crates-io` уже существует"), "{message}");
}

#[test]
fn free_name_is_registered_as_proxy_and_prepared() {
    // Arrange
    let catalog = MemoryCatalog::default();

    // Act
    let prepared = block_on(build(&catalog)).unwrap().unwrap();

    // Assert
    let names: Vec<&str> = prepared
        .cargo
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(names, [NAME]);
    let stored = block_on(catalog.find_repository(Format::Cargo, NAME))
        .unwrap()
        .map(|repository| repository.kind);
    assert_eq!(stored, Some(RepositoryKind::Proxy));
}

#[test]
fn existing_hosted_npm_repository_blocks_startup_naming_the_npm_variable() {
    // Arrange
    let catalog = MemoryCatalog::default();
    block_on(seed_as(&catalog, Format::Npm, RepositoryKind::Hosted));

    // Act
    let result = block_on(build_npm(&catalog));

    // Assert
    let message = result.err().map(|error| error.to_string()).unwrap();
    assert!(message.contains("CRANE_NPM_PROXY_REPOS"), "{message}");
}

#[test]
fn free_npm_name_is_registered_as_npm_proxy() {
    // Arrange
    let catalog = MemoryCatalog::default();

    // Act
    let built = block_on(build_npm(&catalog));

    // Assert
    let stored = block_on(catalog.find_repository(Format::Npm, NAME))
        .unwrap()
        .map(|repository| repository.kind);
    assert!(built.is_ok());
    assert_eq!(stored, Some(RepositoryKind::Proxy));
}
