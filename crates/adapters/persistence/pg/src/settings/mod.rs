//! Настройки и состояние мастера первого запуска в PostgreSQL (20.2).
//!
//! Шифрование секретов - деталь этого адаптера: порт работает с открытым `Secret`.

mod crypto;
mod onboarding;
mod store;

/// Ошибка адаптера в форме, которую несут `Backend`-варианты портов.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub use crypto::{CryptoError, KeyError, SettingsKey};
pub use onboarding::PgOnboarding;
pub use store::PgSettings;
