use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crane_domain::proxy::{EgressRefusal, ProxyKey};
use crane_domain::registry::Sha256;
use crane_ports::proxy::Cached;
use futures_util::FutureExt;
use futures_util::future::{BoxFuture, Shared};

use super::ProxyError;

/// Общий итог выборки: `Shared` отдаёт его клон каждому ждущему.
pub(super) type Flight = Shared<BoxFuture<'static, Result<Cached, Failure>>>;

type Map = Mutex<HashMap<ProxyKey, Flight>>;

/// Исход неудачной выборки. `Clone`, потому что один итог получают все присоединившиеся.
#[derive(Debug, Clone)]
pub(super) enum Failure {
    Refused(EgressRefusal),
    Unavailable,
    DigestMismatch { expected: Sha256, actual: Sha256 },
    TooLarge { limit: u64 },
    Backend(Arc<dyn std::error::Error + Send + Sync>),
}

impl Failure {
    pub(super) fn backend(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Backend(Arc::new(error))
    }
}

impl From<Failure> for ProxyError {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Refused(refusal) => Self::Refused(refusal),
            Failure::Unavailable => Self::UpstreamUnavailable,
            Failure::DigestMismatch { expected, actual } => {
                Self::DigestMismatch { expected, actual }
            }
            Failure::TooLarge { limit } => Self::TooLarge { limit },
            Failure::Backend(source) => Self::Backend(source),
        }
    }
}

/// Нарушенный инвариант данных (запись не того вида под ключом): для клиента это 500.
#[derive(Debug, thiserror::Error)]
#[error("нарушен инвариант кэша: {0}")]
pub(super) struct Invariant(pub(super) &'static str);

/// Схлопывание одинаковых выборок в процессе: параллельные запросы одного ключа делят один полёт.
#[derive(Debug, Default)]
pub(super) struct Flights(Arc<Map>);

fn lock(map: &Map) -> MutexGuard<'_, HashMap<ProxyKey, Flight>> {
    // Отравление не портит карту: каждая операция над ней одно вставка/удаление.
    map.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Flights {
    /// Присоединяет к идущему полёту по ключу или запускает новый через `start`.
    ///
    /// Полёт сам убирает себя из карты по завершении: тогда новый полёт по ключу появляется
    /// только после удаления старого и чужой свежий полёт удалить нельзя. Замок не держится через await.
    pub(super) fn join(
        &self,
        key: ProxyKey,
        start: impl FnOnce() -> BoxFuture<'static, Result<Cached, Failure>>,
    ) -> Flight {
        let mut flights = lock(&self.0);
        if let Some(running) = flights.get(&key) {
            return running.clone();
        }
        let work = start();
        let map = Arc::clone(&self.0);
        let slot = key.clone();
        let flight = async move {
            let outcome = work.await;
            lock(&map).remove(&slot);
            outcome
        }
        .boxed()
        .shared();
        flights.insert(key, flight.clone());
        flight
    }
}
