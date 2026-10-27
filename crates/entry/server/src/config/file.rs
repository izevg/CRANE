//! Необязательный файл TOML под слоем окружения: приоритет env > файл > умолчание.
//!
//! Ключи файла — имена переменных без префикса `CRANE_` в нижнем регистре. Значения
//! приводятся к строкам и ложатся в тот же поиск по имени переменной, поэтому разбор
//! отдельных параметров о файле ничего не знает.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use toml::{Table, Value};

use super::ConfigError;

const CONFIG_PATH: &str = "CRANE_CONFIG";
const PREFIX: &str = "CRANE_";
/// Допустимые ключи. Ключ `config` намеренно отсутствует: путь к файлу задаёт только окружение,
/// иначе файл мог бы подменить сам себя.
const KEYS: [&str; 9] = [
    "http_port",
    "admin_port",
    "public_url",
    "token",
    "token_file",
    "cargo_repos",
    "max_publish_bytes",
    "database_url",
    "blobs",
];
const LIST_KEY: &str = "cargo_repos";

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

fn entry(key: &str, value: &Value) -> Result<(String, String), ConfigError> {
    let known =
        KEYS.iter()
            .find(|known| **known == key)
            .ok_or_else(|| ConfigError::UnknownKey {
                key: key.to_owned(),
            })?;
    let invalid = || ConfigError::FileValue {
        key: known,
        reason: "ожидалась строка или целое число",
    };
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Integer(number) => number.to_string(),
        Value::Array(items) if *known == LIST_KEY => items
            .iter()
            .map(|item| item.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or(ConfigError::FileValue {
                key: known,
                reason: "ожидался массив строк",
            })?
            .join(","),
        _ => return Err(invalid()),
    };
    Ok((format!("{PREFIX}{}", known.to_uppercase()), text))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use super::*;

    fn env_with(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |key| map.get(key).cloned()
    }

    fn file(content: &'static str) -> impl Fn(&Path) -> io::Result<String> + use<> {
        move |path| {
            assert_eq!(path, Path::new("/etc/crane.toml"));
            Ok(content.to_owned())
        }
    }

    fn lookup(
        env: &[(&str, &str)],
        content: &'static str,
    ) -> Result<impl Fn(&str) -> Option<String> + use<>, ConfigError> {
        let mut all = vec![(CONFIG_PATH, "/etc/crane.toml")];
        all.extend_from_slice(env);
        layered(env_with(&all), &file(content))
    }

    fn error_of(env: &[(&str, &str)], content: &'static str) -> ConfigError {
        lookup(env, content).err().unwrap()
    }

    #[test]
    fn without_config_path_there_is_no_file() {
        // Arrange
        let env = env_with(&[("CRANE_HTTP_PORT", "1")]);
        let never = |_: &Path| -> io::Result<String> {
            Err(io::Error::other("не должен читаться"))
        };

        // Act
        let get = layered(env, &never).ok().unwrap();

        // Assert
        assert_eq!(get("CRANE_HTTP_PORT").as_deref(), Some("1"));
        assert_eq!(get("CRANE_ADMIN_PORT"), None);
    }

    #[test]
    fn file_value_is_used_when_env_is_absent() {
        // Arrange
        let get = lookup(&[], "public_url = \"https://file.example\"")
            .ok()
            .unwrap();

        // Act
        let value = get("CRANE_PUBLIC_URL");

        // Assert
        assert_eq!(value.as_deref(), Some("https://file.example"));
    }

    #[test]
    fn env_overrides_file() {
        // Arrange
        let get = lookup(
            &[("CRANE_PUBLIC_URL", "https://env.example")],
            "public_url = \"https://file.example\"",
        )
        .ok()
        .unwrap();

        // Act
        let value = get("CRANE_PUBLIC_URL");

        // Assert
        assert_eq!(value.as_deref(), Some("https://env.example"));
    }

    #[test]
    fn every_known_key_maps_to_its_prefixed_upper_case_variable() {
        // Arrange
        let content = "http_port = 1\nadmin_port = 2\npublic_url = \"u\"\ntoken = \"t\"\n\
                       token_file = \"f\"\ncargo_repos = \"r\"\nmax_publish_bytes = 3\n\
                       database_url = \"d\"\nblobs = \"b\"\n";
        let get = lookup(&[], content).ok().unwrap();

        // Act
        let values = [
            "CRANE_HTTP_PORT",
            "CRANE_ADMIN_PORT",
            "CRANE_PUBLIC_URL",
            "CRANE_TOKEN",
            "CRANE_TOKEN_FILE",
            "CRANE_CARGO_REPOS",
            "CRANE_MAX_PUBLISH_BYTES",
            "CRANE_DATABASE_URL",
            "CRANE_BLOBS",
        ]
        .map(|name| get(name).unwrap());

        // Assert
        assert_eq!(values, ["1", "2", "u", "t", "f", "r", "3", "d", "b"]);
    }

    #[test]
    fn array_of_strings_becomes_comma_list() {
        // Arrange
        let get = lookup(&[], "cargo_repos = [\"a\", \"b\"]").ok().unwrap();

        // Act
        let value = get("CRANE_CARGO_REPOS");

        // Assert
        assert_eq!(value.as_deref(), Some("a,b"));
    }

    #[test]
    fn unknown_key_is_rejected_by_name() {
        // Arrange
        let content = "colour = \"red\"";

        // Act
        let error = error_of(&[], content);

        // Assert
        assert_eq!(
            error,
            ConfigError::UnknownKey {
                key: "colour".to_owned()
            }
        );
    }

    #[test]
    fn config_key_inside_the_file_is_rejected() {
        // Arrange
        let content = "config = \"/other.toml\"";

        // Act
        let error = error_of(&[], content);

        // Assert
        assert_eq!(
            error,
            ConfigError::UnknownKey {
                key: "config".to_owned()
            }
        );
    }

    #[test]
    fn wrong_value_types_are_rejected_without_echoing_values() {
        // Arrange
        let boolean = "token = true";
        let float = "http_port = 1.5";
        let array_for_scalar = "public_url = [\"x\"]";
        let mixed_array = "cargo_repos = [\"a\", 1]";

        // Act
        let errors = [boolean, float, array_for_scalar, mixed_array].map(|c| error_of(&[], c));

        // Assert
        let keys = errors.map(|e| match e {
            ConfigError::FileValue { key, .. } => key,
            other => panic!("ожидалась FileValue: {other:?}"),
        });
        assert_eq!(keys, ["token", "http_port", "public_url", "cargo_repos"]);
    }

    #[test]
    fn parse_error_reports_position_but_not_file_contents() {
        // Arrange
        let content = "http_port = 1\ntoken = \"SECRETVAL\" bad\n";

        // Act
        let error = error_of(&[], content);
        let message = error.to_string();

        // Assert
        assert_eq!(
            error,
            ConfigError::FileParse {
                path: "/etc/crane.toml".to_owned(),
                line: 2,
                column: 21
            }
        );
        assert!(message.contains("/etc/crane.toml"), "{message}");
        assert!(!message.contains("SECRETVAL"), "{message}");
    }

    #[test]
    fn unreadable_file_reports_path_and_reason() {
        // Arrange
        let env = env_with(&[(CONFIG_PATH, "/missing.toml")]);
        let missing = |_: &Path| -> io::Result<String> {
            Err(io::Error::new(io::ErrorKind::NotFound, "нет файла"))
        };

        // Act
        let error = layered(env, &missing).err().unwrap();

        // Assert
        assert_eq!(
            error,
            ConfigError::FileRead {
                path: "/missing.toml".to_owned(),
                reason: "нет файла".to_owned()
            }
        );
    }
}
