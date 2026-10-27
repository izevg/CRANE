use std::future::ready;
use std::time::Duration;

use crane_domain::proxy::ProxyKey;
use crane_ports::proxy::{CacheError, Cached, CachedState, ProxyCache};
use moka::sync::Cache;

/// Бюджет по умолчанию: один документ metadata может весить до 64 MiB.
const DEFAULT_BYTES: u64 = 64 * 1024 * 1024;

/// Условные накладные расходы записи сверх тела: ключ, счётчики, метки времени.
const ENTRY_OVERHEAD_BYTES: u32 = 64;

/// Кэш proxy в процессе для single-node, ограниченный по байтам.
///
/// Вес записи — тело и `ETag` документа плюс условные накладные расходы; при превышении бюджета
/// moka вытесняет записи лениво, поэтому суммарный вес может ненадолго его превышать. Запись
/// тяжелее всего бюджета не удерживается. `keep_for` игнорируется: вытеснение — право бэкенда,
/// свежесть решает use case, поэтому TTL в moka не ставится.
#[derive(Debug)]
pub struct MemoryProxyCache {
    entries: Cache<ProxyKey, Cached>,
}

impl MemoryProxyCache {
    /// Кэш, удерживающий не больше `max_bytes` условного веса записей.
    #[must_use]
    pub fn bounded(max_bytes: u64) -> Self {
        let entries = Cache::builder()
            .max_capacity(max_bytes)
            .weigher(|_key: &ProxyKey, entry: &Cached| weight(entry))
            .build();
        Self { entries }
    }
}

impl Default for MemoryProxyCache {
    fn default() -> Self {
        Self::bounded(DEFAULT_BYTES)
    }
}

fn weight(entry: &Cached) -> u32 {
    let payload = match &entry.state {
        CachedState::Document { body, etag, .. } => body
            .len()
            .saturating_add(etag.as_ref().map_or(0, |etag| etag.as_str().len())),
        CachedState::Blob(_) | CachedState::Missing { .. } => 0,
    };
    u32::try_from(payload).map_or(u32::MAX, |payload| {
        ENTRY_OVERHEAD_BYTES.saturating_add(payload)
    })
}

impl ProxyCache for MemoryProxyCache {
    // `async fn` без `.await` не проходит `clippy::unused_async`, поэтому готовые футуры.
    fn get(
        &self,
        key: &ProxyKey,
    ) -> impl Future<Output = Result<Option<Cached>, CacheError>> + Send {
        ready(Ok(self.entries.get(key)))
    }

    fn put(
        &self,
        key: &ProxyKey,
        entry: Cached,
        _keep_for: Duration,
    ) -> impl Future<Output = Result<(), CacheError>> + Send {
        self.entries.insert(key.clone(), entry);
        ready(Ok(()))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "тесты: падение = провал проверки"
)]
mod tests {
    use std::time::SystemTime;

    use bytes::Bytes;
    use crane_domain::proxy::UpstreamPath;
    use crane_domain::registry::RepositoryId;
    use futures_executor::block_on;
    use uuid::Uuid;

    use super::*;

    const KEEP: Duration = Duration::from_secs(60);

    fn key(n: u128) -> ProxyKey {
        ProxyKey {
            repository: RepositoryId::new(Uuid::from_u128(n)),
            path: UpstreamPath::parse("index/se/rd/serde").unwrap(),
        }
    }

    fn document(body_len: usize) -> Cached {
        Cached {
            fetched_at: SystemTime::UNIX_EPOCH,
            state: CachedState::Document {
                body: Bytes::from(vec![0_u8; body_len]),
                etag: None,
                unchanged: 0,
                max_age: None,
            },
        }
    }

    #[test]
    fn document_heavier_than_budget_is_not_retained() {
        // Arrange
        let cache = MemoryProxyCache::bounded(1_000);
        block_on(cache.put(&key(1), document(2_000), KEEP)).unwrap();
        cache.entries.run_pending_tasks();

        // Act
        let found = block_on(cache.get(&key(1))).unwrap();

        // Assert
        assert!(found.is_none(), "запись тяжелее бюджета удержана");
    }

    #[test]
    fn retained_documents_never_exceed_budget() {
        // Arrange
        let cache = MemoryProxyCache::bounded(500);
        let keys: Vec<ProxyKey> = (1..=10).map(key).collect();
        for key in &keys {
            block_on(cache.put(key, document(100), KEEP)).unwrap();
        }
        cache.entries.run_pending_tasks();

        // Act
        let retained = keys
            .iter()
            .filter(|key| block_on(cache.get(key)).unwrap().is_some())
            .count();

        // Assert
        assert!((1..=3).contains(&retained), "удержано {retained} из 10");
    }
}
