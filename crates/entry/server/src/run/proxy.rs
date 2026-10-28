//! Сборка proxy-репозиториев: один движок на процесс для всех форматов, router на каждое имя.

use std::sync::Arc;

use crane_app::proxy::{ProxyEngine, ProxyLimits};
use crane_app::registry::Registry;
use crane_domain::proxy::{EgressPolicy, Origin};
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_format_cargo::{DL_HOST, INDEX_HOST};
use crane_format_npm::REGISTRY_HOST;
use crane_http::{
    CargoProxyOrigins, EngineUpstreamNames, FormatSettings, ProxyRouters, cargo_proxy_router,
    npm_proxy_router,
};
use crane_ports::proxy::UpstreamNames;
use crane_ports::registry::{BlobStore, Catalog};
use crane_ports::system::IdGen;
use crane_upstream::HttpUpstream;

use super::RunError;
use crate::cache::{self, Backend};
use crate::config::{Proxy, Repos, proxy_var};
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

/// Подготовленные proxy-репозитории: движок и источники, но ещё без routers. Routers нужен
/// `Registry` (защита внутренних имён), а `Registry` нужен источник `UpstreamNames` из того же
/// движка, поэтому сборка идёт в два шага.
pub(super) struct Prepared<B> {
    engine: ProxyEngine<HttpUpstream, Backend, Backend, Arc<B>>,
    cargo: Vec<(String, CargoProxyOrigins)>,
    npm: Vec<(String, Origin)>,
}

impl<B: BlobStore + 'static> Prepared<B> {
    /// Что публичные реестры знают об имени: по первому proxy-репозиторию каждого формата
    /// (хосты upstream у всех одни, ключ кэша по репозиторию, поэтому одного достаточно).
    pub(super) fn upstream_names(&self) -> Arc<dyn UpstreamNames> {
        Arc::new(EngineUpstreamNames::new(
            self.engine.clone(),
            self.cargo.first().map(|(_, origins)| origins.index.clone()),
            self.npm.first().map(|(_, origin)| origin.clone()),
        ))
    }

    /// Routers для каждого имени каждого формата над общим `Registry`.
    pub(super) fn mount<C: Catalog + 'static>(
        self,
        registry: &Arc<Registry<C, Arc<B>>>,
        formats: &FormatSettings,
    ) -> ProxyRouters {
        let cargo = self.cargo.into_iter().map(|(name, origins)| {
            let router = cargo_proxy_router(
                self.engine.clone(),
                Arc::clone(registry),
                origins,
                &formats.cargo,
                &name,
            );
            (name, router)
        });
        let npm = self.npm.into_iter().map(|(name, origin)| {
            let router = npm_proxy_router(
                self.engine.clone(),
                Arc::clone(registry),
                origin,
                &formats.npm,
                &name,
            );
            (name, router)
        });
        ProxyRouters {
            cargo: cargo.collect(),
            npm: npm.collect(),
        }
    }
}

/// Регистрирует репозитории в каталоге и строит движок с источниками каждого имени каждого формата.
///
/// Движок один на все форматы (общие singleflight, кэш и лимиты) и создаётся, только если настроен
/// хотя бы один proxy-репозиторий любого формата: выключенный proxy ничего не стоит (`None`).
pub(super) async fn prepare_proxies<C: Catalog, B: BlobStore + 'static>(
    catalog: &C,
    ids: &dyn IdGen,
    blobs: Arc<B>,
    repos: &Repos,
    proxy: Proxy,
) -> Result<Option<Prepared<B>>, RunError> {
    if !repos.any(RepositoryKind::Proxy) {
        return Ok(None);
    }
    // Хосты upstream зашиты в форматы: proxy ходит только на них, и только по https.
    let policy = EgressPolicy::new([INDEX_HOST, DL_HOST, REGISTRY_HOST], false);
    let index = policy.admit("https", INDEX_HOST, None, "")?;
    let dl = policy.admit("https", DL_HOST, None, "")?;
    let registry = policy.admit("https", REGISTRY_HOST, None, "")?;
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
    let mut cargo = Vec::new();
    for name in repos.names(Format::Cargo, RepositoryKind::Proxy) {
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
        cargo.push((name.clone(), origins));
    }
    let mut npm = Vec::new();
    for name in repos.names(Format::Npm, RepositoryKind::Proxy) {
        let stored = register(catalog, ids, Format::Npm, name).await?;
        // Документы и архивы под одним id: ключи не пересекаются (архивы всегда содержат `/-/`).
        let origin = Origin {
            repository: stored.id,
            base: registry.clone(),
        };
        npm.push((name.clone(), origin));
    }
    Ok(Some(Prepared { engine, cargo, npm }))
}

#[cfg(test)]
#[path = "tests/proxy.rs"]
mod tests;
