//! Сборка proxy-репозиториев crates.io: движок один на процесс, router на каждое имя.

use std::sync::Arc;

use axum::Router;
use crane_app::proxy::{ProxyEngine, ProxyLimits};
use crane_domain::proxy::{EgressPolicy, Origin};
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_format_cargo::{DL_HOST, INDEX_HOST};
use crane_http::{CargoProxyOrigins, CargoSettings, cargo_proxy_router};
use crane_ports::registry::{BlobStore, Catalog};
use crane_ports::system::IdGen;
use crane_upstream::HttpUpstream;

use super::RunError;
use crate::cache;
use crate::config::Cache;
use crate::system::SystemClock;

/// Параметры движка: бэкенд кэша и пределы размера. Отдельная структура держит число аргументов
/// `cargo_proxies` в пределах clippy.
pub(super) struct Engine {
    pub(super) cache: Cache,
    pub(super) limits: ProxyLimits,
}

/// Регистрирует репозитории в каталоге и строит router для каждого имени.
///
/// Без имён возвращает пустой список и не создаёт HTTP-клиент: выключенный proxy ничего не стоит.
pub(super) async fn cargo_proxies<C: Catalog, B: BlobStore + 'static>(
    catalog: &C,
    ids: &dyn IdGen,
    blobs: Arc<B>,
    engine: Engine,
    names: &[String],
    settings: &CargoSettings,
) -> Result<Vec<(String, Router)>, RunError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    // Хосты crates.io зашиты в формат: proxy ходит только на них, и только по https.
    let policy = EgressPolicy::new([INDEX_HOST, DL_HOST], false);
    let index = policy.admit("https", INDEX_HOST, None, "")?;
    let dl = policy.admit("https", DL_HOST, None, "")?;
    // Один бэкенд на оба порта: кэш и lease выбираются вместе (`CRANE_CACHE`).
    let shared = cache::backend(engine.cache);
    let engine = ProxyEngine::new(
        HttpUpstream::new(policy)?,
        shared.clone(),
        shared,
        blobs,
        Arc::new(SystemClock),
        engine.limits,
    );
    let mut routers = Vec::with_capacity(names.len());
    for name in names {
        let stored = catalog
            .ensure_repository(Repository {
                id: RepositoryId::new(ids.generate()),
                format: Format::Cargo,
                kind: RepositoryKind::Proxy,
                name: name.clone(),
            })
            .await?;
        // Каталог идемпотентен по имени и вернёт прежний вид; тихо превратить hosted в proxy нельзя,
        // иначе опубликованные пакеты стали бы недостижимы.
        if stored.kind != RepositoryKind::Proxy {
            return Err(format!(
                "репозиторий `{name}` уже существует как {:?}; CRANE_CARGO_PROXY_REPOS требует свободное имя",
                stored.kind
            )
            .into());
        }
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

    const NAME: &str = "crates-io";

    fn settings() -> CargoSettings {
        CargoSettings {
            public_url: "https://crane.example".to_owned(),
            max_publish_bytes: 1,
        }
    }

    async fn seed(catalog: &MemoryCatalog, kind: RepositoryKind) {
        let ids = SequentialIdGen::default();
        catalog
            .ensure_repository(Repository {
                id: RepositoryId::new(ids.generate()),
                format: Format::Cargo,
                kind,
                name: NAME.to_owned(),
            })
            .await
            .unwrap();
    }

    async fn build(catalog: &MemoryCatalog) -> Result<Vec<(String, Router)>, RunError> {
        cargo_proxies(
            catalog,
            &SequentialIdGen::default(),
            Arc::new(MemoryBlobStore::default()),
            Engine {
                cache: Cache::Memory,
                limits: ProxyLimits::DEFAULT,
            },
            &[NAME.to_owned()],
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
}
