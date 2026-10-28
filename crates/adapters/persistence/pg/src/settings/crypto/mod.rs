//! Ключ и конверт шифрования секретов настроек: чистые функции без БД.

mod envelope;
mod key;

pub use envelope::CryptoError;
pub(super) use envelope::{open, seal};
pub use key::{KeyError, SettingsKey};
