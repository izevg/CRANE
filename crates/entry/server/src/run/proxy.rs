//! Сборка proxy-репозиториев: один движок на процесс для всех форматов, router на каждое имя.

use std::sync::Arc;

use axum::Router;
use crane_app::proxy::{ProxyEngine, ProxyLimits};
use crane_domain::proxy::{EgressPolicy, Origin};
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_format_cargo::{DL_HOST, INDEX_HOST};
use crane_format_npm::REGISTRY_HOST;
use crane_http::{CargoProxyOrigins, CargoSettings, cargo_proxy_router};
use crane_ports::registry::{BlobStore, Catalog};
use crane_ports::system::IdGen;
use crane_upstream::HttpUpstream;

use super::RunError;
use crate::cache;
use crate::config::{Proxy, proxy_var};
use crate::system::SystemClock;

/// Регистрирует proxy-репозиторий; hosted с тем же именем молча превращать в proxy нельзя,
/// иначе опубликованные пакеты стали бы недостижимы.
async fn register<C: Catalog>(
    catalog: &C,
    ids: &dyn IdGen,
    format: Format,
    name: &str,
) -> Result<Repository, RunError> {
    let stored = catalog
        .ensure_repository(Repository {
            id: RepositoryId::new(ids.generate()),
            format,
            kind: RepositoryKind::Proxy,
            name: name.to_owned(),
        })
        .await?;
    // Каталог идемпотентен по имени и вернёт прежний вид.
    if stored.kind != RepositoryKind::Proxy {
        return Err(format!(
            "репозиторий `{name}` уже существует как {:?}; {} требует свободное имя",
            stored.kind,
            proxy_var(format)
        )
        .into());
    }
    Ok(stored)
}

/// Регистрирует репозитории в каталоге и строит router для каждого имени.
///
/// Движок один на все форматы (общие singleflight, кэш и лимиты) и создаётся, только если настроен
/// хотя бы один proxy-репозиторий любого формата: выключенный proxy ничего не стоит.
pub(super) async fn proxies<C: Catalog, B: BlobStore + 'static>(
    catalog: &C,
    ids: &dyn IdGen,
    blobs: Arc<B>,
    proxy: Proxy,
    settings: &CargoSettings,
) -> Result<Vec<(String, Router)>, RunError> {
    if proxy.repos.is_empty() {
        return Ok(Vec::new());
    }
    // Хосты upstream зашиты в форматы: proxy ходит только на них, и только по https.
    let policy = EgressPolicy::new([INDEX_HOST, DL_HOST, REGISTRY_HOST], false);
    let index = policy.admit("https", INDEX_HOST, None, "")?;
    let dl = policy.admit("https", DL_HOST, None, "")?;
    // Один бэкенд на оба порта: кэш и lease выбираются вместе (`CRANE_CACHE`).
    let shared = cache::backend(proxy.cache);
    let engine = ProxyEngine::new(
        HttpUpstream::new(policy)?,
        shared.clone(),
        shared,
        blobs,
        Arc::new(SystemClock),
        ProxyLimits {
            max_artifact_bytes: proxy.max_artifact_bytes,
            ..ProxyLimits::DEFAULT
        },
    );
    let mut routers = Vec::new();
    for name in proxy.repos.names(Format::Cargo) {
        let stored = register(catalog, ids, Format::Cargo, name).await?;
        // Индекс и архивы под одним id: ключи не пересекаются (первый сегмент `crates` не бывает у индекса).
        let origins = CargoProxyOrigins {
            index: Origin {
                repository: stored.id,
                base: index.clone(),
            },
            dl: Origin {
                repository: stored.id,
                base: dl.clone(),
            },
        };
        routers.push((
            name.clone(),
            cargo_proxy_router(engine.clone(), origins, settings, name),
        ));
    }
    // Репозитории npm регистрируются (и проверяются на занятость) уже сейчас; монтирование — отдельным шагом.
    for name in proxy.repos.names(Format::Npm) {
        register(catalog, ids, Format::Npm, name).await?;
    }
    Ok(routers)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use std::future::Future;

    use crane_memory::{MemoryBlobStore, MemoryCatalog, SequentialIdGen};
    use tokio::runtime::Builder;

    use super::*;
    use crate::config::{Cache, FormatRepos};

    const NAME: &str = "crates-io";

    fn settings() -> CargoSettings {
        CargoSettings {
            public_url: "https://crane.example".to_owned(),
            max_publish_bytes: 1,
        }
    }

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

    async fn build(catalog: &MemoryCatalog) -> Result<Vec<(String, Router)>, RunError> {
        proxies(
            catalog,
            &SequentialIdGen::default(),
            Arc::new(MemoryBlobStore::default()),
            Proxy {
                repos: FormatRepos::of(Format::Cargo, vec![NAME.to_owned()]),
                cache: Cache::Memory,
                max_artifact_bytes: ProxyLimits::DEFAULT.max_artifact_bytes,
            },
            &settings(),
        )
        .await
    }

    async fn build_npm(catalog: &MemoryCatalog) -> Result<Vec<(String, Router)>, RunError> {
        proxies(
            catalog,
            &SequentialIdGen::default(),
            Arc::new(MemoryBlobStore::default()),
            Proxy {
                repos: FormatRepos::of(Format::Npm, vec![NAME.to_owned()]),
                cache: Cache::Memory,
                max_artifact_bytes: ProxyLimits::DEFAULT.max_artifact_bytes,
            },
            &settings(),
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
    fn free_name_is_registered_as_proxy_and_mounted() {
        // Arrange
        let catalog = MemoryCatalog::default();

        // Act
        let routers = block_on(build(&catalog)).unwrap();

        // Assert
        let names: Vec<&str> = routers.iter().map(|(name, _)| name.as_str()).collect();
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
}
