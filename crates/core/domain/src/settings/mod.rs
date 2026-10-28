//! Настройки платформы: метаданные ключей, секреты и хранимые значения (только данные, без IO).

mod def;
mod secret;
mod stored;

pub use def::{Kind, Origin, SettingDef, Unit};
pub use secret::{Secret, locator};
pub use stored::{NewValue, Stored, StoredValue, Unreadable, Version};
