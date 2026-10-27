use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use crane_domain::audit::AuditEvent;
use crane_ports::audit::AuditSink;
use crane_ports::system::{Clock, IdGen};
use uuid::Uuid;

/// Тестовые часы: стоят, пока их не сдвинут. Нужны TTL-логике proxy без sleep.
#[derive(Debug)]
pub struct ManualClock(Mutex<SystemTime>);

impl ManualClock {
    #[must_use]
    pub const fn new(now: SystemTime) -> Self {
        Self(Mutex::new(now))
    }

    /// Сдвигает время вперёд.
    ///
    /// Переполнение `SystemTime` на практике недостижимо; в этом случае часы не сдвигаются.
    pub fn advance(&self, by: Duration) {
        // Отравление замка не портит данные: значение меняется одной записью.
        let mut now = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *now = now.checked_add(by).unwrap_or(*now);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> SystemTime {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Детерминированные идентификаторы: 1, 2, 3, ...
#[derive(Debug, Default)]
pub struct SequentialIdGen(AtomicU64);

impl IdGen for SequentialIdGen {
    fn generate(&self) -> Uuid {
        // Счётчику нужна только уникальность, порядок с другой памятью не важен.
        let next = self.0.fetch_add(1, Ordering::Relaxed) + 1;
        Uuid::from_u128(u128::from(next))
    }
}
/// Журнал аудита в памяти: копит события для проверки в тестах.
#[derive(Debug, Default)]
pub struct MemoryAudit {
    events: Mutex<Vec<AuditEvent>>,
}

impl MemoryAudit {
    /// Снимок записанных событий в порядке поступления.
    #[must_use]
    pub fn events(&self) -> Vec<AuditEvent> {
        // Отравление замка не портит данные: push атомарен внутри замка.
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl AuditSink for MemoryAudit {
    fn record(&self, event: AuditEvent) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: SystemTime = SystemTime::UNIX_EPOCH;

    #[test]
    fn now_returns_initial_time_before_any_advance() {
        // Arrange
        let clock = ManualClock::new(START);

        // Act
        let now = clock.now();

        // Assert
        assert_eq!(now, START);
    }

    #[test]
    fn now_reflects_advance() {
        // Arrange
        let clock = ManualClock::new(START);

        // Act
        clock.advance(Duration::from_secs(30));

        // Assert
        assert_eq!(clock.now(), START + Duration::from_secs(30));
    }

    #[test]
    fn consecutive_advances_accumulate() {
        // Arrange
        let clock = ManualClock::new(START);
        clock.advance(Duration::from_secs(30));

        // Act
        clock.advance(Duration::from_secs(12));

        // Assert
        assert_eq!(clock.now(), START + Duration::from_secs(42));
    }
}
