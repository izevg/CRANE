//! Конфигурация: переменные окружения поверх необязательного файла TOML (`CRANE_CONFIG`).
//! Приоритет: env > файл > умолчание.
//!
//! Разбор чистый: источники значений (env, чтение файла) приходят замыканиями, поэтому
//! каждое правило проверяется без обращения к настоящему окружению. Файл (`file`) лишь
//! достраивает поиск по имени переменной, разбор отдельных параметров о нём не знает.

mod file;

use std::fmt;
use std::io;
use std::path::Path;

use crane_domain::access::{Principal, PrincipalKind};
use crane_http::TrustedProxies;
pub(crate) use file::layered;

const DEFAULT_HTTP_PORT: u16 = 8080;
const DEFAULT_ADMIN_PORT: u16 = 9464;
const DEFAULT_REPOS: &str = "internal";
/// Совпадает с прежним логином владельца в ответе cargo `owners`.
const DEFAULT_PRINCIPAL: &str = "crane";
const DEFAULT_MAX_PUBLISH_BYTES: usize = 10 * 1024 * 1024;
const MEMORY: &str = "memory";
/// Подстановка, когда у значения нет читаемой схемы: само значение может быть секретом.
const NO_SCHEME: &str = "(нет схемы)";

const DATABASE_URL: &str = "CRANE_DATABASE_URL";
const BLOBS: &str = "CRANE_BLOBS";
const HTTP_PORT: &str = "CRANE_HTTP_PORT";
const ADMIN_PORT: &str = "CRANE_ADMIN_PORT";
const PUBLIC_URL: &str = "CRANE_PUBLIC_URL";
const TOKEN: &str = "CRANE_TOKEN";
const TOKEN_FILE: &str = "CRANE_TOKEN_FILE";
const CARGO_REPOS: &str = "CRANE_CARGO_REPOS";
const MAX_PUBLISH_BYTES: &str = "CRANE_MAX_PUBLISH_BYTES";
const TOKEN_PRINCIPAL: &str = "CRANE_TOKEN_PRINCIPAL";
const TRUSTED_PROXIES: &str = "CRANE_TRUSTED_PROXIES";

/// Единственный список имён переменных. Ключи файла выводятся из него (`file::variable`),
/// отдельного перечня в файле нет. `CRANE_CONFIG` сюда не входит намеренно.
pub(super) const VARS: [&str; 11] = [
    HTTP_PORT,
    ADMIN_PORT,
    PUBLIC_URL,
    TOKEN,
    TOKEN_FILE,
    TOKEN_PRINCIPAL,
    TRUSTED_PROXIES,
    CARGO_REPOS,
    MAX_PUBLISH_BYTES,
    DATABASE_URL,
    BLOBS,
];

/// Ошибка конфигурации. Значения секретных переменных в тексте не появляются никогда.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConfigError {
    Missing {
        var: &'static str,
    },
    Invalid {
        var: &'static str,
        reason: &'static str,
    },
    Conflict {
        first: &'static str,
        second: &'static str,
    },
    /// В `scheme` только схема URL, без логина, пароля и адреса.
    Unsupported {
        var: &'static str,
        scheme: String,
    },
    TokenFile {
        reason: String,
    },
    UnknownKey {
        key: String,
    },
    FileValue {
        key: String,
        reason: &'static str,
    },
    FileRead {
        path: String,
        reason: String,
    },
    /// Только позиция: сообщение парсера цитирует строку файла, а там может быть токен.
    FileParse {
        path: String,
        line: usize,
        column: usize,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { var } => write!(f, "не задана переменная {var}"),
            Self::Invalid { var, reason } => write!(f, "некорректное значение {var}: {reason}"),
            Self::Conflict { first, second } => {
                write!(f, "переменные {first} и {second} взаимоисключающие")
            }
            Self::Unsupported { var, scheme } => {
                write!(
                    f,
                    "{var}: значение со схемой `{scheme}` пока не поддерживается"
                )
            }
            Self::TokenFile { reason } => {
                write!(f, "не удалось прочитать {TOKEN_FILE}: {reason}")
            }
            Self::UnknownKey { key } => write!(f, "неизвестный ключ `{key}` в файле конфигурации"),
            Self::FileValue { key, reason } => {
                write!(
                    f,
                    "некорректное значение ключа `{key}` в файле конфигурации: {reason}"
                )
            }
            Self::FileRead { path, reason } => {
                write!(f, "не удалось прочитать файл конфигурации {path}: {reason}")
            }
            Self::FileParse { path, line, column } => write!(
                f,
                "ошибка разбора TOML в {path}: строка {line}, столбец {column}"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Catalogs {
    Memory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blobs {
    Memory,
}

/// Выбор хранилищ. Новый бэкенд — новый вариант enum и новая ветка match в `run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Storage {
    pub(crate) catalog: Catalogs,
    pub(crate) blobs: Blobs,
}

impl Storage {
    /// Оба значения обязательны, а `memory` пишется явно: молчаливое хранилище в памяти
    /// в продакшене потеряло бы данные при рестарте.
    pub(crate) fn from_env(var: &impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let catalog = match required(var, DATABASE_URL)?.as_str() {
            MEMORY => Catalogs::Memory,
            other => return Err(unsupported(DATABASE_URL, other)),
        };
        let blobs = match required(var, BLOBS)?.as_str() {
            MEMORY => Blobs::Memory,
            other => return Err(unsupported(BLOBS, other)),
        };
        Ok(Self { catalog, blobs })
    }
}

/// Сырой токен. Без `Debug` и `Display`: случайно попасть в журнал он не может.
pub(crate) struct Secret(String);

impl Secret {
    pub(crate) fn expose(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

pub(crate) struct Settings {
    /// Кому принадлежит статический токен: попадает в аудит и в ответ `owners`.
    pub(crate) principal: Principal,
    pub(crate) trusted_proxies: TrustedProxies,
    pub(crate) http_port: u16,
    pub(crate) admin_port: u16,
    pub(crate) public_url: String,
    pub(crate) token: Secret,
    pub(crate) cargo_repos: Vec<String>,
    pub(crate) max_publish_bytes: usize,
    pub(crate) storage: Storage,
}

impl Settings {
    pub(crate) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        Ok(Self {
            principal: principal(var)?,
            trusted_proxies: trusted_proxies(var)?,
            http_port: number(var, HTTP_PORT, DEFAULT_HTTP_PORT)?,
            admin_port: admin_port(var)?,
            public_url: public_url(var)?,
            token: token(var, read_file)?,
            cargo_repos: repos(var)?,
            max_publish_bytes: max_publish_bytes(var)?,
            storage: Storage::from_env(var)?,
        })
    }
}

fn required(
    var: &impl Fn(&str) -> Option<String>,
    name: &'static str,
) -> Result<String, ConfigError> {
    var(name)
        .filter(|value| !value.is_empty())
        .ok_or(ConfigError::Missing { var: name })
}

/// Отдельно от `Settings`: проба здоровья не должна требовать токен, адрес и хранилище.
pub(crate) fn admin_port(var: &impl Fn(&str) -> Option<String>) -> Result<u16, ConfigError> {
    number(var, ADMIN_PORT, DEFAULT_ADMIN_PORT)
}

fn number<T: std::str::FromStr>(
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

fn max_publish_bytes(var: &impl Fn(&str) -> Option<String>) -> Result<usize, ConfigError> {
    let limit = number(var, MAX_PUBLISH_BYTES, DEFAULT_MAX_PUBLISH_BYTES)?;
    // Нулевой предел отклонял бы любую публикацию.
    if limit == 0 {
        return Err(ConfigError::Invalid {
            var: MAX_PUBLISH_BYTES,
            reason: "предел должен быть больше нуля",
        });
    }
    Ok(limit)
}

fn public_url(var: &impl Fn(&str) -> Option<String>) -> Result<String, ConfigError> {
    let raw = required(var, PUBLIC_URL)?;
    let host = raw
        .strip_prefix("https://")
        .or_else(|| raw.strip_prefix("http://"))
        .ok_or(ConfigError::Invalid {
            var: PUBLIC_URL,
            reason: "адрес должен начинаться с http:// или https://",
        })?;
    // Завершающий '/' срезается: к адресу дописываются пути с ведущим '/'.
    let trimmed = host.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(ConfigError::Invalid {
            var: PUBLIC_URL,
            reason: "в адресе нет хоста",
        });
    }
    Ok(raw.trim_end_matches('/').to_owned())
}

fn token(
    var: &impl Fn(&str) -> Option<String>,
    read_file: &impl Fn(&Path) -> io::Result<String>,
) -> Result<Secret, ConfigError> {
    let value = match (var(TOKEN), var(TOKEN_FILE)) {
        (Some(_), Some(_)) => {
            return Err(ConfigError::Conflict {
                first: TOKEN,
                second: TOKEN_FILE,
            });
        }
        (Some(value), None) => value,
        // Перевод строки в конце файла секрета ставит редактор или `echo`, токеном он не является.
        (None, Some(path)) => read_file(Path::new(&path))
            .map(|content| content.trim_end_matches(['\r', '\n']).to_owned())
            .map_err(|error| ConfigError::TokenFile {
                reason: error.to_string(),
            })?,
        (None, None) => return Err(ConfigError::Missing { var: TOKEN }),
    };
    if value.is_empty() {
        return Err(ConfigError::Invalid {
            var: TOKEN,
            reason: "токен пуст",
        });
    }
    Ok(Secret(value))
}

fn principal(var: &impl Fn(&str) -> Option<String>) -> Result<Principal, ConfigError> {
    let id = var(TOKEN_PRINCIPAL).unwrap_or_else(|| DEFAULT_PRINCIPAL.to_owned());
    let id = id.trim();
    // Идентификатор попадает в журналы: пустой бессмыслен, управляющие символы портят строку.
    if id.is_empty() || id.chars().any(char::is_control) {
        return Err(ConfigError::Invalid {
            var: TOKEN_PRINCIPAL,
            reason: "ожидалось непустое имя без управляющих символов",
        });
    }
    Ok(Principal {
        id: id.to_owned(),
        kind: PrincipalKind::Robot,
    })
}

fn trusted_proxies(var: &impl Fn(&str) -> Option<String>) -> Result<TrustedProxies, ConfigError> {
    TrustedProxies::parse(&var(TRUSTED_PROXIES).unwrap_or_default()).map_err(|_| {
        ConfigError::Invalid {
            var: TRUSTED_PROXIES,
            reason: "ожидался список IP-адресов или CIDR через запятую",
        }
    })
}

fn repos(var: &impl Fn(&str) -> Option<String>) -> Result<Vec<String>, ConfigError> {
    let raw = var(CARGO_REPOS).unwrap_or_else(|| DEFAULT_REPOS.to_owned());
    raw.split(',')
        .map(|item| {
            let item = item.trim();
            if item.is_empty() {
                Err(ConfigError::Invalid {
                    var: CARGO_REPOS,
                    reason: "пустое имя репозитория в списке",
                })
            } else {
                Ok(item.to_owned())
            }
        })
        .collect()
}

fn unsupported(var: &'static str, value: &str) -> ConfigError {
    ConfigError::Unsupported {
        var,
        scheme: scheme_of(value),
    }
}

/// Схема URL. В значении бывают логин и пароль, поэтому наружу уходит только она,
/// и только если похожа на схему.
fn scheme_of(value: &str) -> String {
    match value.split_once("://") {
        Some((scheme, _))
            if !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) =>
        {
            scheme.to_owned()
        }
        _ => NO_SCHEME.to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use std::collections::HashMap;
    use std::net::IpAddr;

    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |key| map.get(key).cloned()
    }

    fn no_files(_: &Path) -> io::Result<String> {
        Err(io::Error::new(io::ErrorKind::NotFound, "нет файла"))
    }

    const MIN: [(&str, &str); 4] = [
        (DATABASE_URL, "memory"),
        (BLOBS, "memory"),
        (PUBLIC_URL, "https://crane.example"),
        (TOKEN, "t"),
    ];

    fn with(extra: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
        let mut all: Vec<_> = MIN
            .iter()
            .filter(|(k, _)| !extra.iter().any(|(ek, _)| ek == k))
            .copied()
            .collect();
        all.extend_from_slice(extra);
        all
    }

    fn parse(pairs: &[(&str, &str)]) -> Result<Settings, ConfigError> {
        Settings::from_env(&env(pairs), &no_files)
    }

    fn error_of(pairs: &[(&str, &str)]) -> ConfigError {
        parse(pairs).err().unwrap()
    }

    #[test]
    fn minimal_env_gets_defaults() {
        // Arrange
        let pairs = with(&[]);

        // Act
        let settings = parse(&pairs).unwrap();

        // Assert
        assert_eq!(settings.http_port, 8080);
        assert_eq!(settings.admin_port, 9464);
        assert_eq!(settings.cargo_repos, ["internal"]);
        assert_eq!(settings.max_publish_bytes, 10 * 1024 * 1024);
        assert_eq!(settings.public_url, "https://crane.example");
        assert_eq!(settings.token.expose(), b"t");
        assert_eq!(
            settings.storage,
            Storage {
                catalog: Catalogs::Memory,
                blobs: Blobs::Memory
            }
        );
    }

    #[test]
    fn explicit_values_override_defaults() {
        // Arrange
        let pairs = with(&[
            (HTTP_PORT, "1"),
            (ADMIN_PORT, "65535"),
            (MAX_PUBLISH_BYTES, "123"),
        ]);

        // Act
        let settings = parse(&pairs).unwrap();

        // Assert
        assert_eq!(
            (
                settings.http_port,
                settings.admin_port,
                settings.max_publish_bytes
            ),
            (1, 65535, 123)
        );
    }

    #[test]
    fn invalid_ports_are_rejected_naming_the_variable() {
        // Arrange
        let too_big = with(&[(HTTP_PORT, "65536")]);
        let text = with(&[(ADMIN_PORT, "abc")]);

        // Act
        let errors = [too_big, text].map(|pairs| error_of(&pairs));

        // Assert
        let reason = "ожидалось целое число в допустимом диапазоне";
        assert_eq!(
            errors,
            [
                ConfigError::Invalid {
                    var: HTTP_PORT,
                    reason
                },
                ConfigError::Invalid {
                    var: ADMIN_PORT,
                    reason
                }
            ]
        );
    }

    #[test]
    fn zero_publish_limit_is_rejected() {
        // Arrange
        let pairs = with(&[(MAX_PUBLISH_BYTES, "0")]);

        // Act
        let error = error_of(&pairs);

        // Assert
        assert!(matches!(
            error,
            ConfigError::Invalid {
                var: MAX_PUBLISH_BYTES,
                ..
            }
        ));
    }

    #[test]
    fn required_variables_report_which_one_is_missing() {
        // Arrange
        let cases = [DATABASE_URL, BLOBS, PUBLIC_URL];

        // Act
        let errors = cases.map(|missing| {
            let pairs: Vec<_> = MIN.iter().filter(|(k, _)| *k != missing).copied().collect();
            error_of(&pairs)
        });

        // Assert
        assert_eq!(
            errors,
            [DATABASE_URL, BLOBS, PUBLIC_URL].map(|var| ConfigError::Missing { var })
        );
    }

    #[test]
    fn public_url_requires_http_scheme_and_host() {
        // Arrange
        let no_scheme = with(&[(PUBLIC_URL, "crane.example")]);
        let ftp = with(&[(PUBLIC_URL, "ftp://crane.example")]);
        let no_host = with(&[(PUBLIC_URL, "https:///")]);

        // Act
        let errors = [no_scheme, ftp, no_host].map(|pairs| error_of(&pairs));

        // Assert
        let invalid = |reason| ConfigError::Invalid {
            var: PUBLIC_URL,
            reason,
        };
        let scheme = "адрес должен начинаться с http:// или https://";
        assert_eq!(
            errors,
            [
                invalid(scheme),
                invalid(scheme),
                invalid("в адресе нет хоста")
            ]
        );
    }

    #[test]
    fn public_url_accepts_http_and_trims_trailing_slashes() {
        // Arrange
        let pairs = with(&[(PUBLIC_URL, "http://127.0.0.1:8080//")]);

        // Act
        let settings = parse(&pairs).unwrap();

        // Assert
        assert_eq!(settings.public_url, "http://127.0.0.1:8080");
    }

    #[test]
    fn token_comes_from_file_without_trailing_newline() {
        // Arrange
        let pairs: Vec<_> = with(&[(TOKEN_FILE, "/run/secrets/token")])
            .into_iter()
            .filter(|(k, _)| *k != TOKEN)
            .collect();
        let read = |path: &Path| {
            (path == Path::new("/run/secrets/token"))
                .then(|| "file-token\r\n".to_owned())
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        };

        // Act
        let settings = Settings::from_env(&env(&pairs), &read).unwrap();

        // Assert
        assert_eq!(settings.token.expose(), b"file-token");
    }

    #[test]
    fn token_file_keeps_inner_whitespace() {
        // Arrange
        let pairs: Vec<_> = with(&[(TOKEN_FILE, "/t")])
            .into_iter()
            .filter(|(k, _)| *k != TOKEN)
            .collect();
        let read = |_: &Path| Ok(" a b \n".to_owned());

        // Act
        let settings = Settings::from_env(&env(&pairs), &read).unwrap();

        // Assert
        assert_eq!(settings.token.expose(), b" a b ");
    }

    #[test]
    fn token_and_token_file_together_conflict() {
        // Arrange
        let pairs = with(&[(TOKEN_FILE, "/t")]);

        // Act
        let error = error_of(&pairs);

        // Assert
        assert_eq!(
            error,
            ConfigError::Conflict {
                first: TOKEN,
                second: TOKEN_FILE
            }
        );
    }

    #[test]
    fn missing_token_is_reported() {
        // Arrange
        let pairs: Vec<_> = MIN.iter().filter(|(k, _)| *k != TOKEN).copied().collect();

        // Act
        let error = error_of(&pairs);

        // Assert
        assert_eq!(error, ConfigError::Missing { var: TOKEN });
    }

    #[test]
    fn empty_tokens_are_rejected_from_env_and_from_file() {
        // Arrange
        let from_env = with(&[(TOKEN, "")]);
        let from_file: Vec<_> = with(&[(TOKEN_FILE, "/t")])
            .into_iter()
            .filter(|(k, _)| *k != TOKEN)
            .collect();
        let read = |_: &Path| Ok("\n".to_owned());

        // Act
        let errors = [from_env, from_file]
            .map(|pairs| Settings::from_env(&env(&pairs), &read).err().unwrap());

        // Assert
        let empty = || ConfigError::Invalid {
            var: TOKEN,
            reason: "токен пуст",
        };
        assert_eq!(errors, [empty(), empty()]);
    }

    #[test]
    fn unreadable_token_file_is_reported_without_the_token() {
        // Arrange
        let pairs: Vec<_> = with(&[(TOKEN_FILE, "/missing")])
            .into_iter()
            .filter(|(k, _)| *k != TOKEN)
            .collect();

        // Act
        let error = error_of(&pairs);

        // Assert
        assert!(matches!(error, ConfigError::TokenFile { .. }));
    }

    #[test]
    fn repos_list_is_split_and_trimmed() {
        // Arrange
        let pairs = with(&[(CARGO_REPOS, " alpha , beta,gamma ")]);

        // Act
        let settings = parse(&pairs).unwrap();

        // Assert
        assert_eq!(settings.cargo_repos, ["alpha", "beta", "gamma"]);
    }

    #[test]
    fn repos_list_rejects_empty_items() {
        // Arrange
        let trailing = with(&[(CARGO_REPOS, "alpha,")]);
        let blank = with(&[(CARGO_REPOS, "alpha, ,beta")]);

        // Act
        let errors = [trailing, blank].map(|pairs| error_of(&pairs));

        // Assert
        let empty = || ConfigError::Invalid {
            var: CARGO_REPOS,
            reason: "пустое имя репозитория в списке",
        };
        assert_eq!(errors, [empty(), empty()]);
    }

    #[test]
    fn unsupported_storage_names_scheme_but_never_the_secret() {
        // Arrange
        let pairs = with(&[(DATABASE_URL, "postgres://u:SECRET@h/db")]);

        // Act
        let error = error_of(&pairs);

        // Assert
        let message = error.to_string();
        assert_eq!(
            error,
            ConfigError::Unsupported {
                var: DATABASE_URL,
                scheme: "postgres".to_owned()
            }
        );
        assert!(message.contains("postgres"), "{message}");
        assert!(!message.contains("SECRET"), "{message}");
    }

    #[test]
    fn unsupported_blobs_value_without_scheme_is_not_echoed() {
        // Arrange
        let pairs = with(&[(BLOBS, "SECRETvalue")]);

        // Act
        let message = error_of(&pairs).to_string();

        // Assert
        assert!(message.contains(BLOBS), "{message}");
        assert!(!message.contains("SECRET"), "{message}");
    }

    #[test]
    fn storage_from_env_does_not_need_token_or_public_url() {
        // Arrange
        let source = env(&[(DATABASE_URL, "memory"), (BLOBS, "memory")]);

        // Act
        let storage = Storage::from_env(&source).unwrap();

        // Assert
        assert_eq!(storage.catalog, Catalogs::Memory);
        assert_eq!(storage.blobs, Blobs::Memory);
    }

    #[test]
    fn scheme_of_weird_prefix_is_hidden() {
        // Arrange
        let value = "p@ss word://x";

        // Act
        let scheme = scheme_of(value);

        // Assert
        assert_eq!(scheme, NO_SCHEME);
    }

    fn file_lookup(
        env: &[(&str, &str)],
        content: &'static str,
    ) -> impl Fn(&str) -> Option<String> + use<> {
        let mut all = vec![("CRANE_CONFIG", "/etc/crane.toml")];
        all.extend_from_slice(env);
        layered(self::env(&all), &move |_: &Path| Ok(content.to_owned()))
            .ok()
            .unwrap()
    }

    #[test]
    fn token_in_file_conflicts_with_token_file_in_env() {
        // Arrange
        let get = file_lookup(
            &[
                (DATABASE_URL, "memory"),
                (BLOBS, "memory"),
                (PUBLIC_URL, "https://x.example"),
                (TOKEN_FILE, "/t"),
            ],
            "token = \"file-token\"\n",
        );

        // Act
        let error = Settings::from_env(&get, &no_files).err().unwrap();

        // Assert
        assert_eq!(
            error,
            ConfigError::Conflict {
                first: TOKEN,
                second: TOKEN_FILE
            }
        );
    }

    #[test]
    fn token_principal_defaults_to_crane_robot_and_can_be_overridden() {
        // Arrange
        let cases = [with(&[]), with(&[(TOKEN_PRINCIPAL, " ci-bot ")])];

        // Act
        let principals = cases.map(|pairs| parse(&pairs).unwrap().principal);

        // Assert
        let robot = |id: &str| Principal {
            id: id.to_owned(),
            kind: PrincipalKind::Robot,
        };
        assert_eq!(principals, [robot("crane"), robot("ci-bot")]);
    }

    #[test]
    fn blank_or_control_character_principal_is_rejected() {
        // Arrange
        let cases = [
            with(&[(TOKEN_PRINCIPAL, "  ")]),
            with(&[(TOKEN_PRINCIPAL, "a\nb")]),
        ];

        // Act
        let errors = cases.map(|pairs| error_of(&pairs));

        // Assert
        let invalid = || ConfigError::Invalid {
            var: TOKEN_PRINCIPAL,
            reason: "ожидалось непустое имя без управляющих символов",
        };
        assert_eq!(errors, [invalid(), invalid()]);
    }

    #[test]
    fn trusted_proxies_default_to_none_and_follow_the_list() {
        // Arrange
        let cases = [with(&[]), with(&[(TRUSTED_PROXIES, "10.0.0.0/8, ::1")])];
        let peers: [IpAddr; 3] = ["10.1.2.3", "::1", "203.0.113.1"].map(|p| p.parse().unwrap());

        // Act
        let trusted = cases.map(|pairs| {
            let settings = parse(&pairs).unwrap();
            peers.map(|peer| settings.trusted_proxies.contains(peer))
        });

        // Assert
        assert_eq!(trusted, [[false; 3], [true, true, false]]);
    }

    #[test]
    fn malformed_trusted_proxies_are_rejected() {
        // Arrange
        let pairs = with(&[(TRUSTED_PROXIES, "10.0.0.0/33")]);

        // Act
        let error = error_of(&pairs);

        // Assert
        assert_eq!(
            error,
            ConfigError::Invalid {
                var: TRUSTED_PROXIES,
                reason: "ожидался список IP-адресов или CIDR через запятую"
            }
        );
    }

    #[test]
    fn admin_port_needs_nothing_else_and_defaults() {
        // Arrange
        let empty = env(&[]);
        let custom = env(&[(ADMIN_PORT, "19464")]);
        let broken = env(&[(ADMIN_PORT, "x")]);

        // Act
        let results = [empty, custom, broken].map(|source| admin_port(&source));

        // Assert
        assert_eq!(
            results,
            [
                Ok(9464),
                Ok(19464),
                Err(ConfigError::Invalid {
                    var: ADMIN_PORT,
                    reason: "ожидалось целое число в допустимом диапазоне"
                })
            ]
        );
    }
}
