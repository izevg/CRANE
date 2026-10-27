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
use std::str::FromStr;

use crane_domain::access::{Principal, PrincipalKind};
use crane_http::TrustedProxies;
pub(crate) use file::layered;
use sqlx::postgres::PgConnectOptions;
use url::Url;

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
const DATABASE_URL_FILE: &str = "CRANE_DATABASE_URL_FILE";
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
const CARGO_PROXY_REPOS: &str = "CRANE_CARGO_PROXY_REPOS";

/// Единственный список имён переменных. Ключи файла выводятся из него (`file::variable`),
/// отдельного перечня в файле нет. `CRANE_CONFIG` сюда не входит намеренно.
pub(super) const VARS: [&str; 13] = [
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
    DATABASE_URL_FILE,
    BLOBS,
    CARGO_PROXY_REPOS,
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
    /// `var` — имя переменной `*_FILE`; причина ввода-вывода не содержит содержимого файла.
    SecretFile {
        var: &'static str,
        reason: String,
    },
    /// Ключ запроса `CRANE_BLOBS` вне списка несекретных параметров; значение в ошибку не попадает.
    BlobsKey {
        key: String,
    },
    /// Каталог в PostgreSQL над blob в памяти после рестарта ссылался бы на потерянные blob.
    MixedStorage,
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
            Self::SecretFile { var, reason } => {
                write!(f, "не удалось прочитать {var}: {reason}")
            }
            Self::BlobsKey { key } => write!(
                f,
                "{BLOBS}: ключ `{key}` не из списка несекретных параметров; учётные данные и их \
                 поставщики S3 задаются только переменными AWS_*"
            ),
            Self::MixedStorage => write!(
                f,
                "{DATABASE_URL} и {BLOBS}: memory нельзя смешивать с постоянным хранилищем"
            ),
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

/// Выбор хранилищ. Один режим на оба, а не два независимых выбора: смешанная пара
/// непредставима. Новый бэкенд blob (GCS, Azure) этот тип не меняет: схему разбирает `from_url`.
pub(crate) enum Storage {
    /// Данные теряются при рестарте; только оба хранилища вместе.
    Memory,
    /// Каталог и CAS переживают рестарт одинаково, иначе каталог ссылался бы на потерянные blob.
    Durable { database: Database, blobs: Url },
}

impl Storage {
    /// Оба значения обязательны, а `memory` пишется явно: молчаливое хранилище в памяти
    /// в продакшене потеряло бы данные при рестарте.
    pub(crate) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        let database = secret(var, read_file, DATABASE_URL, DATABASE_URL_FILE)?;
        let blobs = required(var, BLOBS)?;
        match (database.expose() == MEMORY, blobs == MEMORY) {
            (true, true) => Ok(Self::Memory),
            (false, false) => Ok(Self::Durable {
                database: Database::parse(&database)?,
                blobs: blobs_url(&blobs)?,
            }),
            _ => Err(ConfigError::MixedStorage),
        }
    }
}

/// Параметры подключения к PostgreSQL. Строка с паролем разбирается сразу и тут же
/// уничтожается, дальше живут только разобранные параметры. Без `Debug`: пароль в журнал не попадёт.
pub(crate) struct Database(Box<PgConnectOptions>);

impl Database {
    fn parse(url: &Secret) -> Result<Self, ConfigError> {
        let value = url.expose();
        // `PgConnectOptions` не проверяет схему: `mysql://…` он принял бы как PostgreSQL.
        if !(value.starts_with("postgres://") || value.starts_with("postgresql://")) {
            return Err(unsupported(DATABASE_URL, value));
        }
        // Текст ошибки sqlx цитирует часть URL (например, значение `sslmode`), поэтому он не выводится.
        PgConnectOptions::from_str(value)
            .map(|options| Self(Box::new(options)))
            .map_err(|_| ConfigError::Invalid {
                var: DATABASE_URL,
                reason: "не разобран как URL postgres://",
            })
    }

    pub(crate) fn options(&self) -> &PgConnectOptions {
        &self.0
    }
}

fn blobs_url(value: &str) -> Result<Url, ConfigError> {
    // Ошибка разбора и сам URL не выводятся: в запросе `s3://` бывают ключи доступа.
    let url = Url::parse(value).map_err(|_| ConfigError::Invalid {
        var: BLOBS,
        reason: "ожидалось memory, file:///абсолютный/путь или s3://бакет",
    })?;
    match url.scheme() {
        "file" | "s3" => {}
        scheme => {
            return Err(ConfigError::Unsupported {
                var: BLOBS,
                scheme: scheme.to_owned(),
            });
        }
    }
    // Ключи проверяет адаптер теми же правилами, что разбирает: в URL остаются только
    // несекретные параметры, поэтому ни URL, ни текст ошибки не процитируют секрет.
    match url
        .query_pairs()
        .find(|(key, value)| !crane_objstore::is_public_s3_param(key, value))
    {
        Some((key, _)) => Err(ConfigError::BlobsKey {
            key: key.into_owned(),
        }),
        None => Ok(url),
    }
}

/// Секрет (токен, URL базы). Без `Debug` и `Display`: случайно попасть в журнал он не может.
pub(crate) struct Secret(String);

impl Secret {
    pub(crate) fn expose(&self) -> &str {
        &self.0
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
    /// Имена proxy-репозиториев crates.io; пустой список — proxy выключен.
    pub(crate) cargo_proxy_repos: Vec<String>,
    pub(crate) max_publish_bytes: usize,
    pub(crate) storage: Storage,
}

impl Settings {
    pub(crate) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        let cargo_repos = repos(var)?;
        let cargo_proxy_repos = proxy_repos(var, &cargo_repos)?;
        Ok(Self {
            principal: principal(var)?,
            trusted_proxies: trusted_proxies(var)?,
            http_port: number(var, HTTP_PORT, DEFAULT_HTTP_PORT)?,
            admin_port: admin_port(var)?,
            public_url: public_url(var)?,
            token: secret(var, read_file, TOKEN, TOKEN_FILE)?,
            cargo_repos,
            cargo_proxy_repos,
            max_publish_bytes: max_publish_bytes(var)?,
            storage: Storage::from_env(var, read_file)?,
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

/// Секрет из `name` или из файла `file_name` (Docker/Kubernetes secrets). Ровно один из двух.
fn secret(
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

/// Имя попадает в путь `/cargo/<имя>`, поэтому набор символов ограничен: `{`, `}` и `/` сломали бы маршрутизацию.
fn proxy_name_is_valid(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Пустая переменная (compose передаёт `""`) и отсутствующая — пустой список.
fn proxy_repos(
    var: &impl Fn(&str) -> Option<String>,
    hosted: &[String],
) -> Result<Vec<String>, ConfigError> {
    let raw = var(CARGO_PROXY_REPOS).unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = Vec::new();
    for item in raw.split(',') {
        let name = item.trim();
        let reason = if name.is_empty() {
            Some("пустое имя репозитория в списке")
        } else if !proxy_name_is_valid(name) {
            Some("имя может содержать только буквы, цифры, `-`, `_` и `.`")
        } else if hosted.iter().any(|hosted| hosted == name) {
            Some("имя совпадает с hosted-репозиторием")
        } else if names.iter().any(|seen| seen == name) {
            Some("имя повторяется в списке")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(ConfigError::Invalid {
                var: CARGO_PROXY_REPOS,
                reason,
            });
        }
        names.push(name.to_owned());
    }
    Ok(names)
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
    fn proxy_repos_list_is_parsed_and_empty_values_mean_no_proxy() {
        // Arrange
        let cases = [
            with(&[]),
            with(&[(CARGO_PROXY_REPOS, "")]),
            with(&[(CARGO_PROXY_REPOS, "crates-io")]),
            with(&[(CARGO_PROXY_REPOS, " crates-io , mirror.2 ")]),
        ];

        // Act
        let actual = cases.map(|pairs| parse(&pairs).unwrap().cargo_proxy_repos);

        // Assert
        assert_eq!(
            actual,
            [
                Vec::<String>::new(),
                Vec::new(),
                vec!["crates-io".to_owned()],
                vec!["crates-io".to_owned(), "mirror.2".to_owned()],
            ]
        );
    }

    #[test]
    fn invalid_proxy_repos_are_rejected_with_reason() {
        // Arrange
        let cases = [
            with(&[(CARGO_PROXY_REPOS, "crates-io,,other")]),
            with(&[(CARGO_PROXY_REPOS, "internal")]),
            with(&[(CARGO_PROXY_REPOS, "a,a")]),
            with(&[(CARGO_PROXY_REPOS, "a{b}")]),
            with(&[(CARGO_PROXY_REPOS, "a/b")]),
        ];

        // Act
        let actual = cases.map(|pairs| error_of(&pairs));

        // Assert
        let invalid = |reason| ConfigError::Invalid {
            var: CARGO_PROXY_REPOS,
            reason,
        };
        assert_eq!(
            actual,
            [
                invalid("пустое имя репозитория в списке"),
                invalid("имя совпадает с hosted-репозиторием"),
                invalid("имя повторяется в списке"),
                invalid("имя может содержать только буквы, цифры, `-`, `_` и `.`"),
                invalid("имя может содержать только буквы, цифры, `-`, `_` и `.`"),
            ]
        );
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
        assert_eq!(settings.token.expose(), "t");
        assert!(matches!(settings.storage, Storage::Memory));
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
        assert_eq!(settings.token.expose(), "file-token");
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
        assert_eq!(settings.token.expose(), " a b ");
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
            reason: "значение пусто",
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
        assert!(matches!(
            error,
            ConfigError::SecretFile {
                var: TOKEN_FILE,
                ..
            }
        ));
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
    fn unsupported_database_names_scheme_but_never_the_secret() {
        // Arrange
        let pairs = with(&[
            (DATABASE_URL, "mysql://u:SECRET@h/db"),
            (BLOBS, "file:///b"),
        ]);

        // Act
        let error = error_of(&pairs);

        // Assert
        let message = error.to_string();
        assert_eq!(
            error,
            ConfigError::Unsupported {
                var: DATABASE_URL,
                scheme: "mysql".to_owned()
            }
        );
        assert!(message.contains("mysql"), "{message}");
        assert!(!message.contains("SECRET"), "{message}");
    }

    #[test]
    fn unparsable_database_url_error_never_contains_the_password() {
        // Arrange
        // sqlx цитирует значение `sslmode` в тексте ошибки: пароль в нём покажет, что маскировка снята.
        let urls = [
            "postgres://u:pw@h/db?sslmode=SECRETPW",
            "postgres://u:SECRETPW@h:notaport/db",
        ];

        // Act
        let messages = urls.map(|url| {
            Database::parse(&Secret(url.to_owned()))
                .err()
                .map(|error| error.to_string())
        });

        // Assert
        let leaked = messages.map(|message| message.is_none_or(|text| text.contains("SECRETPW")));
        assert_eq!(leaked, [false; 2]);
    }

    #[test]
    fn unsupported_blobs_scheme_is_named_without_the_rest_of_the_value() {
        // Arrange
        let pairs = with(&[
            (DATABASE_URL, "postgres://u:pw@h/db"),
            (BLOBS, "gcs://SECRETbucket?key=SECRETkey"),
        ]);

        // Act
        let message = error_of(&pairs).to_string();

        // Assert
        assert!(message.contains("gcs"), "{message}");
        assert!(!message.contains("SECRET"), "{message}");
    }

    #[test]
    fn invalid_blobs_value_is_rejected_without_echoing_it() {
        // Arrange
        let pairs = with(&[(BLOBS, "SECRETvalue")]);

        // Act
        let message = error_of(&pairs).to_string();

        // Assert
        assert!(message.contains(BLOBS), "{message}");
        assert!(!message.contains("SECRET"), "{message}");
    }

    /// Форма результата для таблицы: сравнивать `Storage` целиком нельзя, секрет не `PartialEq`.
    fn shape(result: Result<Storage, ConfigError>) -> String {
        match result {
            Ok(Storage::Memory) => "memory".to_owned(),
            Ok(Storage::Durable { database, blobs }) => {
                format!(
                    "durable {}/{} {}",
                    database.options().get_host(),
                    database.options().get_database().unwrap_or_default(),
                    blobs.scheme()
                )
            }
            Err(error) => format!("error {error}"),
        }
    }

    #[test]
    fn storage_mode_follows_the_pair_of_database_and_blobs() {
        // Arrange
        let cases = [
            ("memory", "memory"),
            ("postgres://u:pw@h/db", "file:///data/blobs"),
            (
                "postgresql://u:pw@h/db",
                "s3://bucket/p?endpoint=http://s3:8333",
            ),
            ("memory", "file:///data/blobs"),
            ("postgres://u:pw@h/db", "memory"),
        ];

        // Act
        let shapes = cases.map(|(database, blobs)| {
            shape(Storage::from_env(
                &env(&[(DATABASE_URL, database), (BLOBS, blobs)]),
                &no_files,
            ))
        });

        // Assert
        let mixed = "error CRANE_DATABASE_URL и CRANE_BLOBS: memory нельзя смешивать с постоянным хранилищем";
        assert_eq!(
            shapes,
            [
                "memory",
                "durable h/db file",
                "durable h/db s3",
                mixed,
                mixed
            ]
        );
    }

    #[test]
    fn blobs_query_accepts_only_public_keys_and_names_the_rejected_one() {
        // Arrange
        let cases = [
            "s3://b?endpoint=http://x:8333&aws_allow_http=true",
            "s3://b?aws_server_side_encryption=aws:kms&aws_sse_kms_key_id=alias/crane",
            "s3://b?aws_secret_access_key=SECRETKEY",
            "s3://b?aws_session_token=SECRETKEY",
            "s3://b?aws_sse_customer_key_base64=SECRETKEY",
            "s3://b?endpoint=http://u:SECRETKEY@x:8333",
            "s3://b?foo=1",
        ];

        // Act
        let results = cases.map(|blobs| {
            blobs_url(blobs)
                .err()
                .map_or_else(|| "принято".to_owned(), |error| error.to_string())
        });

        // Assert
        let rejected = |key: &str| {
            format!(
                "CRANE_BLOBS: ключ `{key}` не из списка несекретных параметров; учётные данные и их \
                 поставщики S3 задаются только переменными AWS_*"
            )
        };
        assert_eq!(
            results,
            [
                "принято".to_owned(),
                "принято".to_owned(),
                rejected("aws_secret_access_key"),
                rejected("aws_session_token"),
                rejected("aws_sse_customer_key_base64"),
                rejected("endpoint"),
                rejected("foo"),
            ]
        );
    }

    #[test]
    fn database_url_file_is_read_without_trailing_newline() {
        // Arrange
        let source = env(&[
            (DATABASE_URL_FILE, "/run/secrets/db"),
            (BLOBS, "file:///data/blobs"),
        ]);
        let read = |path: &Path| {
            (path == Path::new("/run/secrets/db"))
                .then(|| "postgres://u:pw@h/db\r\n".to_owned())
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        };

        // Act
        let storage = Storage::from_env(&source, &read);

        // Assert
        assert_eq!(shape(storage), "durable h/db file");
    }

    #[test]
    fn database_url_and_its_file_together_conflict() {
        // Arrange
        let source = env(&[
            (DATABASE_URL, "memory"),
            (DATABASE_URL_FILE, "/db"),
            (BLOBS, "memory"),
        ]);

        // Act
        let error = Storage::from_env(&source, &no_files).err().unwrap();

        // Assert
        assert_eq!(
            error,
            ConfigError::Conflict {
                first: DATABASE_URL,
                second: DATABASE_URL_FILE
            }
        );
    }

    #[test]
    fn storage_from_env_does_not_need_token_or_public_url() {
        // Arrange
        let source = env(&[(DATABASE_URL, "memory"), (BLOBS, "memory")]);

        // Act
        let storage = Storage::from_env(&source, &no_files).unwrap();

        // Assert
        assert!(matches!(storage, Storage::Memory));
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
