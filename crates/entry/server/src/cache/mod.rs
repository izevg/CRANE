//! Кэш proxy и координатор выборок: что реплики делят между собой.
//!
//! Single-node (`CRANE_CACHE=memory`) — ничего: кэш ограниченный, в процессе, координировать
//! выборки не с кем. Несколько реплик — Valkey: общий кэш (L2), L1 в каждой реплике и lease.

use std::future::ready;
use std::sync::Arc;
use std::time::Duration;

use crane_domain::proxy::ProxyKey;
use crane_memory::MemoryProxyCache;
use crane_ports::proxy::{CacheError, Cached, Claim, FetchLeases, Lease, LeaseError, ProxyCache};
use futures_util::future::Either;

use crate::config::Cache;

/// Бюджет L1 в байтах веса записей (тела документов): общий для обоих режимов.
/// 256 мегабайт.
pub(crate) const L1_BYTES: u64 = 256 * 1024 * 1024;

/// Один тип на оба порта: кэш и lease выбираются вместе, смешанная пара непредставима.
#[derive(Clone, Debug)]
pub(crate) enum Backend {
    Alone(Arc<MemoryProxyCache>),
    Valkey(crane_valkey::Valkey),
}

/// Для режима Valkey требует tokio-рантайм: адаптер запускает фоновые задачи.
#[allow(
    dead_code,
    reason = "сборку ProxyEngine с этим бэкендом делает run/mod.rs (п. 3 v0.2.0); снять при интеграции"
)]
pub(crate) fn backend(cache: Cache) -> Backend {
    match cache {
        Cache::Memory => {
            tracing::info!(cache = "memory", "кэш proxy выбран");
            Backend::Alone(Arc::new(MemoryProxyCache::bounded(L1_BYTES)))
        }
        Cache::Valkey(endpoint) => {
            tracing::info!(cache = "valkey", "кэш proxy выбран");
            Backend::Valkey(crane_valkey::Valkey::start(endpoint, L1_BYTES))
        }
    }
}

impl ProxyCache for Backend {
    async fn get(&self, key: &ProxyKey) -> Result<Option<Cached>, CacheError> {
        match self {
            Self::Alone(cache) => cache.get(key).await,
            Self::Valkey(valkey) => valkey.get(key).await,
        }
    }

    async fn put(
        &self,
        key: &ProxyKey,
        entry: Cached,
        keep_for: Duration,
    ) -> Result<(), CacheError> {
        match self {
            Self::Alone(cache) => cache.put(key, entry, keep_for).await,
            Self::Valkey(valkey) => valkey.put(key, entry, keep_for).await,
        }
    }
}

impl FetchLeases for Backend {
    async fn claim(
        &self,
        key: &ProxyKey,
        lease: Duration,
        patience: Duration,
    ) -> Result<Claim, LeaseError> {
        match self {
            // Выборки внутри процесса схлопывает app (Flights), координировать не с кем.
            // Токен не нужен: release в этом режиме ничего не снимает.
            Self::Alone(_) => Ok(Claim::Lead(Lease::new(key.clone(), 0))),
            Self::Valkey(valkey) => valkey.claim(key, lease, patience).await,
        }
    }

    // Не `async fn`: обёртка отложила бы `Valkey::release` до первого poll и сломала бы
    // eager-контракт порта (`Held::drop` в app бросает future неопрошенным).
    fn release(&self, lease: Lease) -> impl Future<Output = Result<(), LeaseError>> + Send {
        match self {
            Self::Alone(_) => Either::Left(ready(Ok(()))),
            Self::Valkey(valkey) => Either::Right(valkey.release(lease)),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use crane_domain::proxy::UpstreamPath;
    use crane_domain::registry::RepositoryId;
    use futures_util::FutureExt;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn alone_backend_leads_even_a_held_key() {
        // Arrange
        let backend = Backend::Alone(Arc::new(MemoryProxyCache::default()));
        let key = ProxyKey {
            repository: RepositoryId::new(Uuid::from_u128(1)),
            path: UpstreamPath::parse("index/se/rd/serde").unwrap(),
        };
        let (lease, patience) = (Duration::from_secs(30), Duration::from_secs(1));
        let _held = backend.claim(&key, lease, patience).now_or_never();

        // Act
        let second = backend.claim(&key, lease, patience).now_or_never();

        // Assert
        assert!(matches!(second, Some(Ok(Claim::Lead(_)))));
    }
}
