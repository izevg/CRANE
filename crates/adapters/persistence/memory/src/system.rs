use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use crane_ports::system::{Clock, IdGen};
use uuid::Uuid;

/// Часы, застывшие на заданном моменте.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock(SystemTime);

impl FixedClock {
    #[must_use]
    pub const fn new(now: SystemTime) -> Self {
        Self(now)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> SystemTime {
        self.0
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
