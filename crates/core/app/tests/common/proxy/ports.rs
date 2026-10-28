//! Порты для «реплик»: общий кэш, общие leases и их отказы.

use std::future::{Future, ready};
use std::sync::Arc;
use std::time::Duration;

use crane_domain::proxy::ProxyKey;
use crane_memory::{ManualClock, MemoryFetchLeases, MemoryProxyCache};
use crane_ports::proxy::{CacheError, Cached, Claim, FetchLeases, Lease, LeaseError, ProxyCache};

/// Порты не реализованы для `Arc`: общий между «репликами» кэш оборачиваем.
#[derive(Debug, Clone, Default)]
pub(crate) struct SharedCache(pub(crate) Arc<MemoryProxyCache>);

impl ProxyCache for SharedCache {
    fn get(
        &self,
        key: &ProxyKey,
    ) -> impl Future<Output = Result<Option<Cached>, CacheError>> + Send {
        self.0.get(key)
    }

    fn put(
        &self,
        key: &ProxyKey,
        entry: Cached,
        keep_for: Duration,
    ) -> impl Future<Output = Result<(), CacheError>> + Send {
        self.0.put(key, entry, keep_for)
    }
}

/// Кэш, который всегда отказывает.
#[derive(Debug)]
pub(crate) struct BrokenCache;

impl ProxyCache for BrokenCache {
    fn get(
        &self,
        _key: &ProxyKey,
    ) -> impl Future<Output = Result<Option<Cached>, CacheError>> + Send {
        ready(Err(CacheError::Backend("кэш лежит".into())))
    }

    fn put(
        &self,
        _key: &ProxyKey,
        _entry: Cached,
        _keep_for: Duration,
    ) -> impl Future<Output = Result<(), CacheError>> + Send {
        ready(Err(CacheError::Backend("кэш лежит".into())))
    }
}

/// Общие leases; `down` моделирует недоступный координатор.
#[derive(Debug, Clone)]
pub(crate) struct SharedLeases {
    inner: Arc<MemoryFetchLeases>,
    down: bool,
}

impl SharedLeases {
    pub(crate) fn new(clock: Arc<ManualClock>) -> Self {
        Self {
            inner: Arc::new(MemoryFetchLeases::new(clock)),
            down: false,
        }
    }

    pub(crate) fn down(clock: Arc<ManualClock>) -> Self {
        Self {
            down: true,
            ..Self::new(clock)
        }
    }
}

impl FetchLeases for SharedLeases {
    async fn claim(
        &self,
        key: &ProxyKey,
        lease: Duration,
        patience: Duration,
    ) -> Result<Claim, LeaseError> {
        if self.down {
            return Err(LeaseError::Backend("координатор лежит".into()));
        }
        self.inner.claim(key, lease, patience).await
    }

    // Не `async fn`: обёртка обязана сохранить eager-снятие порта, иначе брошенный future ничего не снимет.
    fn release(&self, lease: Lease) -> impl Future<Output = Result<(), LeaseError>> + Send {
        self.inner.release(lease)
    }
}
