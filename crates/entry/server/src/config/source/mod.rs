//! Источники значений: поиск по имени переменной (`layered`), слой БД по ключу (`StoredLayer`),
//! схема настроек для консоли, секреты, обязательные и числовые параметры. О том, какие
//! параметры бывают, этот модуль ничего не знает.

mod file;
mod schema;
mod stored;

use std::io;
use std::path::Path;

use crane_domain::settings::{Kind, Secret, SettingDef};

use super::ConfigError;
pub(crate) use file::layered;
pub(crate) use schema::ConfigSchema;
pub(crate) use stored::StoredLayer;

/// Префикс переменных; ключ настройки — имя без него в нижнем регистре.
const PREFIX: &str = "CRANE_";

/// Определение ключа с умолчаниями: виден в консоли, без единицы. Отличия — через `..setting(…)`.
pub(super) const fn setting(
    group: &'static str,
    key: &'static str,
    env_var: &'static str,
    kind: Kind,
) -> SettingDef {
    SettingDef {
        key,
        env_var,
        group,
        kind,
        unit: None,
        console: true,
    }
}

/// Ключ настройки по имени переменной.
pub(super) fn key_of(env_var: &str) -> String {
    env_var
        .strip_prefix(PREFIX)
        .unwrap_or(env_var)
        .to_ascii_lowercase()
}

pub(super) fn required(
    var: &impl Fn(&str) -> Option<String>,
    name: &'static str,
) -> Result<String, ConfigError> {
    var(name)
        .filter(|value| !value.is_empty())
        .ok_or(ConfigError::Missing { var: name })
}

pub(super) fn number<T: std::str::FromStr>(
    var: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: T,
) -> Result<T, ConfigError> {
    var(name).map_or(Ok(default), |raw| {
        raw.parse().map_err(|_| ConfigError::Invalid {
            var: name,
            reason: "ожидалось целое число в допустимом диапазоне",
        })
    })
}

/// Секрет из `name` или из файла `file_name` (Docker/Kubernetes secrets). Ровно один из двух.
pub(super) fn secret(
    var: &impl Fn(&str) -> Option<String>,
    read_file: &impl Fn(&Path) -> io::Result<String>,
    name: &'static str,
    file_name: &'static str,
) -> Result<Secret, ConfigError> {
    let value = match (var(name), var(file_name)) {
        (Some(_), Some(_)) => {
            return Err(ConfigError::Conflict {
                first: name,
                second: file_name,
            });
        }
        (Some(value), None) => value,
        // Перевод строки в конце файла секрета ставит редактор или `echo`, секретом он не является.
        (None, Some(path)) => read_file(Path::new(&path))
            .map(|content| content.trim_end_matches(['\r', '\n']).to_owned())
            .map_err(|error| ConfigError::SecretFile {
                var: file_name,
                reason: error.to_string(),
            })?,
        (None, None) => return Err(ConfigError::Missing { var: name }),
    };
    if value.is_empty() {
        return Err(ConfigError::Invalid {
            var: name,
            reason: "значение пусто",
        });
    }
    Ok(Secret::new(value))
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
