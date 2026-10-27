//! Настоящие часы и источник идентификаторов: недетерминизм живёт в composition root.

use std::time::SystemTime;

use crane_ports::system::{Clock, IdGen};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// UUID v7 упорядочены по времени: индексы по первичным ключам остаются локальными.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UuidV7Ids;

impl IdGen for UuidV7Ids {
    fn generate(&self) -> Uuid {
        Uuid::now_v7()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_distinct_version_7() {
        // Arrange
        let ids = UuidV7Ids;

        // Act
        let generated = [ids.generate(), ids.generate()];

        // Assert
        assert_ne!(generated[0], generated[1]);
        assert_eq!(generated.map(|id| id.get_version_num()), [7, 7]);
    }

    #[test]
    fn clock_is_after_unix_epoch() {
        // Arrange
        let clock = SystemClock;

        // Act
        let now = clock.now();

        // Assert
        assert!(now > SystemTime::UNIX_EPOCH);
    }
}
