//! In-memory адаптер портов для тестов ядра.
//!
//! Намеренно не «умнее» PostgreSQL-адаптера: не проверяет рёбра state machine
//! и не валидирует метаданные. Поведение фиксирует контракт из `crane_ports::contract`.

mod proxy;
mod registry;
mod system;

pub use proxy::{MemoryFetchLeases, MemoryProxyCache};
pub use registry::{MemoryBlobStore, MemoryCatalog};
pub use system::{ManualClock, MemoryAudit, SequentialIdGen};
