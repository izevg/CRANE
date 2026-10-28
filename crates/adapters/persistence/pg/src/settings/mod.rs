//! Настройки и состояние мастера первого запуска в PostgreSQL (20.2).
//!
//! Шифрование секретов - деталь этого адаптера: порт работает с открытым `Secret`.

mod crypto;
mod db;
mod onboarding;
mod store;

pub use crypto::{CryptoError, KeyError, SettingsKey};
pub use onboarding::PgOnboarding;
pub use store::PgSettings;
