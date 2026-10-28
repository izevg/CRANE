//! Результаты движка: что отдаётся вызывающему и как это попадает в журнал доступа.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use crane_domain::proxy::EgressRefusal;
use crane_domain::registry::Digests;
use crane_ports::registry::ByteStream;

use super::ProxyError;

/// Метка `result` журнала доступа (16.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheResult {
    Hit,
    Miss,
    /// Отдано просроченное (stale-while-revalidate), обновление идёт в фоне.
    Stale,
    /// Upstream не ответил, отдана устаревшая запись (заголовок `Warning`).
    StaleOnError(UpstreamFailure),
}

/// Почему upstream не ответил. Отказ SSRF не прячется за stale-ответом: это событие для `SecOps`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpstreamFailure {
    #[error("upstream отклонён политикой egress")]
    Refused(#[source] EgressRefusal),
    #[error("upstream недоступен")]
    Unavailable,
    #[error("upstream ограничил запросы")]
    Throttled,
}

/// Ответ движка с фактами для журнала доступа и заголовков `Age`/`Warning`.
pub struct Served<T> {
    pub body: T,
    pub result: CacheResult,
    pub age: Duration,
    /// Фоновое обновление, которое entry обязан исполнить (`spawn`); `None`, если обновлять нечего.
    pub revalidation: Option<Revalidation>,
}

impl<T> fmt::Debug for Served<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Тело и future не Debug и не должны попадать в журналы.
        f.debug_struct("Served")
            .field("result", &self.result)
            .field("age", &self.age)
            .field("revalidation", &self.revalidation.is_some())
            .finish_non_exhaustive()
    }
}

/// Артефакт из CAS.
pub struct Fetched {
    pub digests: Digests,
    pub body: ByteStream,
}

impl fmt::Debug for Fetched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fetched")
            .field("digests", &self.digests)
            .finish_non_exhaustive()
    }
}

/// Фоновая ревалидация. Ленивая: полёт начинается (или присоединяется к идущему) только при первом опросе,
/// поэтому брошенная ревалидация ничего не стоит и не плодит выборок.
pub struct Revalidation(pub(super) Pin<Box<dyn Future<Output = Result<(), ProxyError>> + Send>>);

impl Future for Revalidation {
    type Output = Result<(), ProxyError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.as_mut().poll(cx)
    }
}

impl fmt::Debug for Revalidation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Revalidation").finish_non_exhaustive()
    }
}
