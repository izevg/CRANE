use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_domain::proxy::ProxyKey;

use crate::registry::Digests;

/// Непрозрачный валидатор upstream для условных запросов (`If-None-Match`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ETag(String);

impl ETag {
    #[must_use]
    pub const fn new(raw: String) -> Self {
        Self(raw)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Запись proxy-кэша: только факты (когда получена, что получено, сколько раз подряд не менялась).
///
/// Сроки не запечены: их выводит политика domain при чтении, поэтому смена конфигурации
/// действует на лежащие записи.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cached {
    pub fetched_at: SystemTime,
    pub state: CachedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedState {
    /// Ответ upstream есть. `unchanged` — сколько обновлений подряд тело не менялось (адаптивный TTL).
    Found {
        body: CachedBody,
        etag: Option<ETag>,
        unchanged: u32,
    },
    /// Upstream ответил 404. `misses` — сколько раз подряд (рост negative TTL).
    Missing { misses: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedBody {
    /// Сырое тело metadata upstream; переписывание ссылок — дело формата при отдаче.
    Document(Bytes),
    /// Артефакт лежит в CAS; запись хранит только дайджесты.
    Blob(Digests),
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("ошибка кэша proxy")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Кэш записей proxy. Не источник истины: use case трактует ошибку `get` как промах,
/// ошибку `put` игнорирует.
///
/// Реализации: moka (L1, в процессе) и Valkey (L2) за одним трейтом.
pub trait ProxyCache: Send + Sync {
    fn get(
        &self,
        key: &ProxyKey,
    ) -> impl Future<Output = Result<Option<Cached>, CacheError>> + Send;

    /// `keep_for` — сколько записи имеет смысл жить в бэкенде (TTL ключа Valkey); считает use case
    /// из политики. Вытеснение — право бэкенда, а не обязанность: свежесть решает use case.
    fn put(
        &self,
        key: &ProxyKey,
        entry: Cached,
        keep_for: Duration,
    ) -> impl Future<Output = Result<(), CacheError>> + Send;
}
