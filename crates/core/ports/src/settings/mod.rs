//! Порты настроек и первого запуска (20.2). Шифрование секретов — деталь адаптера хранения,
//! порт работает с открытым `Secret`.

mod onboarding;
mod schema;
mod store;

pub use onboarding::{OnboardingError, OnboardingStore};
pub use schema::{Invalid, InvalidKey, SettingsSchema};
pub use store::{SettingsError, SettingsStore};
