#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use std::future::Future;

use crane_memory::{MemoryAudit, MemoryBlobStore, MemoryCatalog, SequentialIdGen};
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
        .repos
        .iter()
        .map(|(_, name, _)| name.as_str())
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

#[test]
fn engine_limits_take_the_configured_artifact_cap_and_keep_other_defaults() {
    // Arrange
    let configured = 1_234;

    // Act
    let limits = limits(configured);

    // Assert
    assert_eq!(
        limits,
        ProxyLimits {
            max_artifact_bytes: 1_234,
            ..ProxyLimits::DEFAULT
        }
    );
}

#[test]
fn no_proxy_repositories_means_no_engine() {
    // Arrange
    let catalog = MemoryCatalog::default();
    let repos = Repos::of(Format::Cargo, RepositoryKind::Hosted, vec![NAME.to_owned()]);

    // Act
    let prepared = block_on(prepare_proxies(
        &catalog,
        &SequentialIdGen::default(),
        Arc::new(MemoryBlobStore::default()),
        &repos,
        Proxy {
            cache: Cache::Memory,
            max_artifact_bytes: ProxyLimits::DEFAULT.max_artifact_bytes,
        },
    ));

    // Assert
    assert!(matches!(prepared, Ok(None)));
}

#[test]
fn mount_builds_a_router_for_every_prepared_name_of_each_format() {
    // Arrange
    let catalog = MemoryCatalog::default();
    let cargo = block_on(build(&catalog)).unwrap().unwrap();
    let npm = block_on(build_npm(&MemoryCatalog::default()))
        .unwrap()
        .unwrap();
    let registry = |prepared: &Prepared<MemoryBlobStore>| {
        Arc::new(Registry::new(
            MemoryCatalog::default(),
            Arc::new(MemoryBlobStore::default()),
            Arc::new(SystemClock),
            Arc::new(SequentialIdGen::default()),
            Arc::new(MemoryAudit::default()),
            prepared.upstream_names(),
        ))
    };
    let formats = FormatSettings {
        public_url: "http://crane.test".to_owned(),
        max_publish_bytes: 1_024,
    };
    let (cargo_registry, npm_registry) = (registry(&cargo), registry(&npm));

    // Act
    let mounted = [
        cargo.mount(&cargo_registry, &formats).unwrap(),
        npm.mount(&npm_registry, &formats).unwrap(),
    ];

    // Assert
    let names = mounted.map(|routers| {
        routers
            .into_iter()
            .map(|(format, name, _)| (format, name))
            .collect::<Vec<_>>()
    });
    let expected = |format| vec![(format, NAME.to_owned())];
    assert_eq!(names, [expected(Format::Cargo), expected(Format::Npm)]);
}

#[test]
fn mount_refuses_a_format_without_proxy_support() {
    // Arrange
    let mut prepared = block_on(build(&MemoryCatalog::default())).unwrap().unwrap();
    let repository = prepared.repos[0].2;
    prepared.repos = vec![(Format::PyPi, NAME.to_owned(), repository)];
    let registry = Arc::new(Registry::new(
        MemoryCatalog::default(),
        Arc::new(MemoryBlobStore::default()),
        Arc::new(SystemClock),
        Arc::new(SequentialIdGen::default()),
        Arc::new(MemoryAudit::default()),
        prepared.upstream_names(),
    ));
    let formats = FormatSettings {
        public_url: "http://crane.test".to_owned(),
        max_publish_bytes: 1_024,
    };

    // Act
    let result = prepared.mount(&registry, &formats);

    // Assert
    let message = result.err().map(|error| error.to_string());
    assert_eq!(
        message.as_deref(),
        Some("proxy формата pypi не поддерживается")
    );
}
