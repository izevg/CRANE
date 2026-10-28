//! Сборка proxy-репозиториев: один движок на процесс для всех форматов, router на каждое имя.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use crane_app::proxy::{ProxyEngine, ProxyLimits};
use crane_app::registry::Registry;
use crane_domain::proxy::{EgressPolicy, Origin};
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_http::{
    EngineUpstreamNames, FormatSettings, ProxySource, presence_origin, proxy_router, upstream_hosts,
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
    policy: EgressPolicy,
    repos: Vec<(Format, String, RepositoryId)>,
    /// Источник документа для проверки имён: по первому proxy-репозиторию каждого формата
    /// (хосты upstream у всех одни, ключ кэша по репозиторию, поэтому одного достаточно).
    sources: BTreeMap<Format, Origin>,
}

impl<B: BlobStore + 'static> Prepared<B> {
    /// Что публичные реестры знают об имени.
    pub(super) fn upstream_names(&self) -> Arc<dyn UpstreamNames> {
        Arc::new(EngineUpstreamNames::new(
            self.engine.clone(),
            self.sources.clone(),
        ))
    }

    /// Routers для каждого имени каждого формата над общим `Registry`.
    pub(super) fn mount<C: Catalog + 'static>(
        self,
        registry: &Arc<Registry<C, Arc<B>>>,
        settings: &FormatSettings,
    ) -> Result<Vec<(Format, String, Router)>, RunError> {
        let mut routers = Vec::with_capacity(self.repos.len());
        for (format, name, repository) in self.repos {
            let source = ProxySource {
                repository,
                name: &name,
                policy: &self.policy,
            };
            let router = proxy_router(
                format,
                self.engine.clone(),
                Arc::clone(registry),
                &source,
                settings,
            )?
            .ok_or_else(|| format!("proxy формата {} не поддерживается", format.as_str()))?;
            routers.push((format, name, router));
        }
        Ok(routers)
    }
}

/// Лимиты движка: настраивается только предел артефакта, остальное берётся по умолчанию.
fn limits(max_artifact_bytes: u64) -> ProxyLimits {
    ProxyLimits {
        max_artifact_bytes,
        ..ProxyLimits::DEFAULT
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
    repos_config: &Repos,
    proxy: Proxy,
) -> Result<Option<Prepared<B>>, RunError> {
    if !repos_config.any(RepositoryKind::Proxy) {
        return Ok(None);
    }
    // Хосты upstream зашиты в форматы: proxy ходит только на них, и только по https.
    let policy = EgressPolicy::new(
        Format::ALL
            .iter()
            .flat_map(|format| upstream_hosts(*format).iter().copied()),
        false,
    );
    // Один бэкенд на оба порта: кэш и lease выбираются вместе (`CRANE_CACHE`).
    let shared = cache::backend(proxy.cache);
    let engine = ProxyEngine::new(
        HttpUpstream::new(policy.clone())?,
        shared.clone(),
        shared,
        blobs,
        Arc::new(SystemClock),
        limits(proxy.max_artifact_bytes),
    );
    let mut repos = Vec::new();
    let mut sources = BTreeMap::new();
    for format in Format::ALL {
        for name in repos_config.names(format, RepositoryKind::Proxy) {
            let stored = register(catalog, ids, format, name).await?;
            let source = ProxySource {
                repository: stored.id,
                name,
                policy: &policy,
            };
            if let Some(origin) = presence_origin(format, &source)? {
                sources.entry(format).or_insert(origin);
            }
            repos.push((format, name.clone(), stored.id));
        }
    }
    Ok(Some(Prepared {
        engine,
        policy,
        repos,
        sources,
    }))
}

#[cfg(test)]
#[path = "tests/proxy.rs"]
mod tests;
