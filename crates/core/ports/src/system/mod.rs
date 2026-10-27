//! Недетерминированные источники: время и идентификаторы.

use std::time::SystemTime;

use uuid::Uuid;

/// Источник текущего времени.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
}

/// Источник новых идентификаторов.
pub trait IdGen: Send + Sync {
    fn generate(&self) -> Uuid;
}
