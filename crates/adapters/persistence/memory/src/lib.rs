//! In-memory адаптер портов для тестов ядра.
//!
//! Намеренно не «умнее» PostgreSQL-адаптера: не проверяет рёбра state machine
//! и не валидирует метаданные. Поведение фиксирует контракт из `crane_ports::contract`.

mod blob;
mod catalog;
mod system;

pub use blob::MemoryBlobStore;
pub use catalog::MemoryCatalog;
pub use system::{FixedClock, SequentialIdGen};
