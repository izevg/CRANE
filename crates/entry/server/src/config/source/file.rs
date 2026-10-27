//! Необязательный файл TOML под слоем окружения: приоритет env > файл > умолчание.
//!
//! Ключи файла выводятся из списка переменных (`config::vars`): имя без префикса `CRANE_` в нижнем
//! регистре. Значения приводятся к строкам (массив склеивается запятыми) и ложатся в тот же
//! поиск по имени переменной, поэтому разбор отдельных параметров о файле ничего не знает.
//! `CRANE_CONFIG` в списке переменных нет, так что ключа `config` в файле быть не может: путь к файлу
//! задаёт только окружение, иначе файл мог бы подменить сам себя.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use toml::{Table, Value};

use crate::config::{ConfigError, vars};

const CONFIG_PATH: &str = "CRANE_CONFIG";
const PREFIX: &str = "CRANE_";
/// Поиск значения по имени переменной: сначала окружение, потом файл.
pub(crate) fn layered<E: Fn(&str) -> Option<String>, R: Fn(&Path) -> io::Result<String>>(
    env: E,
    read_file: &R,
) -> Result<impl Fn(&str) -> Option<String> + use<E, R>, ConfigError> {
    let file = match env(CONFIG_PATH).filter(|path| !path.is_empty()) {
        Some(path) => load(Path::new(&path), read_file)?,
        None => HashMap::new(),
    };
    Ok(move |name: &str| env(name).or_else(|| file.get(name).cloned()))
}

fn load(
    path: &Path,
    read_file: &impl Fn(&Path) -> io::Result<String>,
) -> Result<HashMap<String, String>, ConfigError> {
    let shown = path.display().to_string();
    let content = read_file(path).map_err(|error| ConfigError::FileRead {
        path: shown.clone(),
        reason: error.to_string(),
    })?;
    // Display ошибки toml цитирует строку источника, а там может быть токен:
    // наружу уходит только позиция.
    let table: Table = content.parse().map_err(|error: toml::de::Error| {
        let (line, column) = error
            .span()
            .map_or((1, 1), |span| position(&content, span.start));
        ConfigError::FileParse {
            path: shown,
            line,
            column,
        }
    })?;
    table
        .into_iter()
        .map(|(key, value)| entry(key.as_str(), &value))
        .collect()
}

fn position(content: &str, offset: usize) -> (usize, usize) {
    let before = content.get(..offset).unwrap_or(content);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |tail| tail.chars().count())
        + 1;
    (line, column)
}

/// Имя переменной по ключу файла.
fn variable(key: &str) -> Option<&'static str> {
    vars().find(|var| {
        var.strip_prefix(PREFIX)
            .is_some_and(|name| name.to_ascii_lowercase() == key)
    })
}

fn entry(key: &str, value: &Value) -> Result<(String, String), ConfigError> {
    let var = variable(key).ok_or_else(|| ConfigError::UnknownKey {
        key: key.to_owned(),
    })?;
    let invalid = |reason| ConfigError::FileValue {
        key: key.to_owned(),
        reason,
    };
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Integer(number) => number.to_string(),
        Value::Array(items) => items
            .iter()
            .map(|item| item.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| invalid("ожидался массив строк"))?
            .join(","),
        _ => {
            return Err(invalid("ожидалась строка, целое число или массив строк"));
        }
    };
    Ok((var.to_owned(), text))
}

#[cfg(test)]
#[path = "tests/file.rs"]
mod tests;
