use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crane_domain::proxy::ProxyKey;
use crane_ports::proxy::Cached;
use futures_util::FutureExt;
use futures_util::future::{BoxFuture, Shared, WeakShared};

use super::ProxyError;

pub(super) type Outcome = Result<Cached, ProxyError>;

/// Общий итог выборки: `Shared` отдаёт его клон каждому ждущему.
pub(super) type Flight = Shared<BoxFuture<'static, Outcome>>;

/// В карте слабые ссылки: полёт живёт, пока его кто-то ждёт. Отменённый всеми ждущими полёт
/// уничтожается вместе с upstream-соединением и staging, а не ждёт следующего запроса.
type Slots = HashMap<ProxyKey, WeakShared<BoxFuture<'static, Outcome>>>;

type Map = Mutex<Slots>;

/// Нарушенный инвариант данных (запись не того вида под ключом): для клиента это 500.
#[derive(Debug, thiserror::Error)]
#[error("нарушен инвариант кэша: {0}")]
pub(super) struct Invariant(pub(super) &'static str);

/// Схлопывание одинаковых выборок в процессе: параллельные запросы одного ключа делят один полёт.
#[derive(Debug, Default)]
pub(super) struct Flights(Arc<Map>);

fn lock(map: &Map) -> MutexGuard<'_, Slots> {
    // Отравление не портит карту: каждая операция над ней одно вставка/удаление.
    map.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Flights {
    /// Присоединяет к идущему полёту по ключу или запускает новый через `start`.
    ///
    /// Полёт сам убирает себя из карты по завершении. Мёртвая (брошенная) запись заменяется новым полётом:
    /// lease брошенного лидера истечёт сам, ради этого он и существует. Замок не держится через await.
    pub(super) fn join(
        &self,
        key: ProxyKey,
        start: impl FnOnce() -> BoxFuture<'static, Outcome>,
    ) -> Flight {
        let mut flights = lock(&self.0);
        if let Some(running) = flights.get(&key).and_then(WeakShared::upgrade) {
            return running;
        }
        let work = start();
        let map = Arc::clone(&self.0);
        let slot = key.clone();
        let flight = async move {
            let outcome = work.await;
            // Завершиться может только живой полёт, а заменить его в слоте можно лишь после гибели
            // всех сильных ссылок: чужой новый полёт здесь удалить нельзя.
            lock(&map).remove(&slot);
            outcome
        }
        .boxed()
        .shared();
        if let Some(weak) = flight.downgrade() {
            flights.insert(key, weak);
        }
        flight
    }
}
