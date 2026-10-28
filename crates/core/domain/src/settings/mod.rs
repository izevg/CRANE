//! Настройки платформы: метаданные ключей, секреты и хранимые значения (только данные, без IO).

mod def;
mod secret;
mod stored;

pub use def::{Kind, Origin, SettingDef, Unit};
pub use secret::Secret;
pub use stored::{Stored, StoredValue, Version};
