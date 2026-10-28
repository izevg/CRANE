use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::sync::oneshot;

/// Ждущие одного ключа: идентификатор регистрации и отправитель пробуждения.
type Entries = Vec<(u64, oneshot::Sender<()>)>;

/// Ждущие чужого лидера: имя ключа записи -> отправители пробуждения.
#[derive(Default)]
pub(crate) struct Waiters {
    next_id: AtomicU64,
    by_name: Mutex<HashMap<String, Entries>>,
}

/// Регистрация ждущего. `Drop` снимает её из карты, поэтому ушедший по `patience`
/// (или отменённый) ждущий не оставляет мусор.
pub(crate) struct Registration<'a> {
    waiters: &'a Waiters,
    name: String,
    id: u64,
    receiver: oneshot::Receiver<()>,
}

impl Waiters {
    // Замок берётся только в синхронных функциях: guard не живёт через await.
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Entries>> {
        self.by_name.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn register(&self, name: &str) -> Registration<'_> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.lock()
            .entry(name.to_owned())
            .or_default()
            .push((id, sender));
        Registration {
            waiters: self,
            name: name.to_owned(),
            id,
            receiver,
        }
    }

    /// Будит всех ждущих ключа `name`.
    pub(crate) fn wake(&self, name: &str) {
        let woken = self.lock().remove(name).unwrap_or_default();
        for (_, sender) in woken {
            // Ждущий мог уйти: это не ошибка.
            let _ = sender.send(());
        }
    }

    /// Будит всех ждущих: сообщения могли потеряться, и ждать больше нечего.
    pub(crate) fn wake_all(&self) {
        let woken = std::mem::take(&mut *self.lock());
        for (_, sender) in woken.into_values().flatten() {
            let _ = sender.send(());
        }
    }

    fn unregister(&self, name: &str, id: u64) {
        let mut by_name = self.lock();
        if let Some(entries) = by_name.get_mut(name) {
            entries.retain(|(entry_id, _)| *entry_id != id);
            if entries.is_empty() {
                by_name.remove(name);
            }
        }
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }
}

impl Registration<'_> {
    /// Ждёт пробуждения не дольше `patience`. Любой исход (сигнал, закрытие, таймаут)
    /// означает одно: пора перечитать кэш.
    pub(crate) async fn wait(&mut self, patience: Duration) {
        let _ = tokio::time::timeout(patience, &mut self.receiver).await;
    }
}

impl Drop for Registration<'_> {
    fn drop(&mut self) {
        self.waiters.unregister(&self.name, self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_releases_only_waiters_of_that_name() {
        // Arrange
        let waiters = Waiters::default();
        let mut target = waiters.register("a");
        let mut bystander = waiters.register("b");

        // Act
        waiters.wake("a");

        // Assert
        assert_eq!(
            (target.receiver.try_recv(), bystander.receiver.try_recv()),
            (Ok(()), Err(oneshot::error::TryRecvError::Empty)),
            "разбужен ждущий своего ключа, чужой остался ждать"
        );
    }

    #[test]
    fn wake_all_releases_waiters_of_every_name() {
        // Arrange
        let waiters = Waiters::default();
        let mut first = waiters.register("a");
        let mut second = waiters.register("b");

        // Act
        waiters.wake_all();

        // Assert
        assert_eq!(
            (first.receiver.try_recv(), second.receiver.try_recv()),
            (Ok(()), Ok(()))
        );
    }

    #[test]
    fn dropped_registration_keeps_other_waiters_of_the_same_name() {
        // Arrange
        let waiters = Waiters::default();
        let mut kept = waiters.register("a");
        let gone = waiters.register("a");
        drop(gone);

        // Act
        waiters.wake("a");

        // Assert
        assert_eq!(
            kept.receiver.try_recv(),
            Ok(()),
            "снимается только своя регистрация"
        );
    }

    #[test]
    fn last_dropped_registration_removes_the_entry() {
        // Arrange
        let waiters = Waiters::default();
        let only = waiters.register("a");

        // Act
        drop(only);

        // Assert
        assert!(waiters.is_empty(), "пустая запись не копится в карте");
    }

    #[tokio::test]
    async fn wait_lasts_for_the_whole_patience_without_a_wake() {
        // Arrange
        let waiters = Waiters::default();
        let mut registration = waiters.register("a");
        let patience = Duration::from_millis(100);
        let started = std::time::Instant::now();

        // Act
        registration.wait(patience).await;

        // Assert
        assert!(
            started.elapsed() >= patience,
            "без сигнала ждём всё терпение"
        );
    }
}
