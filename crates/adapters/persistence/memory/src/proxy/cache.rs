use std::collections::HashMap;
use std::future::ready;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use crane_domain::proxy::ProxyKey;
use crane_ports::proxy::{CacheError, Cached, ProxyCache};

/// Кэш proxy в памяти.
///
/// `keep_for` игнорируется: вытеснение — право бэкенда, а не обязанность, свежесть решает use case.
#[derive(Debug, Default)]
pub struct MemoryProxyCache {
    entries: Mutex<HashMap<ProxyKey, Cached>>,
}

impl ProxyCache for MemoryProxyCache {
    // `async fn` без `.await` не проходит `clippy::unused_async`, поэтому готовые футуры.
    fn get(
        &self,
        key: &ProxyKey,
    ) -> impl Future<Output = Result<Option<Cached>, CacheError>> + Send {
        // Отравление замка не портит данные: каждая операция над картой атомарна внутри замка.
        let found = self
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .cloned();
        ready(Ok(found))
    }

    fn put(
        &self,
        key: &ProxyKey,
        entry: Cached,
        _keep_for: Duration,
    ) -> impl Future<Output = Result<(), CacheError>> + Send {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key.clone(), entry);
        ready(Ok(()))
    }
}
