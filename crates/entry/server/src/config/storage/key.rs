//! Ключ шифрования секретов в БД: `CRANE_SETTINGS_KEY` или `CRANE_SETTINGS_KEY_FILE`.

use std::io;
use std::path::Path;

use crane_pg::SettingsKey;

use super::{SETTINGS_KEY, SETTINGS_KEY_FILE};
use crate::config::ConfigError;
use crate::config::source::secret;

/// Необязателен: без него секреты в БД не записываются и не читаются. Только из слоя
/// env + файл (из консоли ключ не меняют). При каталоге в памяти ключ — мёртвая конфигурация.
/// Ни содержимое, ни его фрагменты в ошибку не попадают.
pub(crate) fn settings_key(
    var: &impl Fn(&str) -> Option<String>,
    read_file: &impl Fn(&Path) -> io::Result<String>,
    persistent: bool,
) -> Result<Option<SettingsKey>, ConfigError> {
    if var(SETTINGS_KEY).is_none() && var(SETTINGS_KEY_FILE).is_none() {
        return Ok(None);
    }
    if !persistent {
        return Err(ConfigError::SettingsKeyWithMemory);
    }
    let encoded = secret(var, read_file, SETTINGS_KEY, SETTINGS_KEY_FILE)?;
    SettingsKey::from_base64(&encoded)
        .map(Some)
        .map_err(|reason| ConfigError::SettingsKey { reason })
}

#[cfg(test)]
#[path = "tests/key.rs"]
mod tests;
