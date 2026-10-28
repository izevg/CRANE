//! Порты bounded context «proxy»: upstream, кэш записей, межрепликовые lease.

mod cache;
mod leases;
mod names;

use std::fmt;
use std::time::Duration;

use crane_domain::proxy::{EgressRefusal, UpstreamBase, UpstreamPath};

use crate::registry::ByteStream;

pub use cache::{CacheError, Cached, CachedState, ETag, ProxyCache};
pub use leases::{Claim, FetchLeases, Lease, LeaseError};
pub use names::{NoUpstream, Presence, UpstreamNames};

#[derive(Debug)]
pub struct UpstreamRequest<'a> {
    pub base: &'a UpstreamBase,
    pub path: &'a UpstreamPath,
    /// Медиатип для заголовка `Accept`: константа формата; `None` — заголовок не ставится.
    pub accept: Option<&'static str>,
    pub if_none_match: Option<&'a ETag>,
}

pub enum UpstreamResponse {
    /// RFC 9111 §4.3.4: 304 обновляет срок свежести, поэтому он может нести новый `max_age`.
    NotModified {
        max_age: Option<Duration>,
    },
    Found {
        body: ByteStream,
        etag: Option<ETag>,
        /// Срок свежести из `Cache-Control` upstream, целые секунды; `None` — срок не объявлен.
        max_age: Option<Duration>,
    },
    NotFound,
}

// Поток не Debug (как `Download` в app), поэтому вручную и без тела.
impl fmt::Debug for UpstreamResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotModified { max_age } => f
                .debug_struct("NotModified")
                .field("max_age", max_age)
                .finish(),
            Self::Found { etag, max_age, .. } => f
                .debug_struct("Found")
                .field("etag", etag)
                .field("max_age", max_age)
                .finish_non_exhaustive(),
            Self::NotFound => f.write_str("NotFound"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpstreamError {
    /// SSRF guard отказал (redirect-hop или адрес после DNS): событие для `SecOps`.
    #[error("upstream отклонён политикой egress")]
    Refused(#[source] EgressRefusal),
    /// Сеть, таймаут, 5xx и любой ответ, кроме 200/304/404/410/451 и 429/403.
    #[error("upstream недоступен")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// Upstream отказывается обслуживать нас: rate limit (429) или, у публичных реестров,
    /// блокировка WAF или IP (403). Это не снятие пакета: оно приходит как 404/410/451.
    #[error("upstream ограничил запросы")]
    Throttled,
}

/// Транспорт к upstream. Адаптер (reqwest) обязан применять `EgressPolicy::check_address`
/// в своём DNS-резолвере (до connect) и `check_target` к каждому redirect-hop: только так
/// проверяется адрес, к которому реально подключаются.
///
/// Контракта нет: поведение задаёт удалённая сторона; `app` тестируется скриптованным фейком.
pub trait Upstream: Send + Sync {
    fn fetch(
        &self,
        request: UpstreamRequest<'_>,
    ) -> impl Future<Output = Result<UpstreamResponse, UpstreamError>> + Send;
}
