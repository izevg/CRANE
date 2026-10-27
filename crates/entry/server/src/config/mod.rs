//! Конфигурация: переменные окружения поверх необязательного файла TOML (`CRANE_CONFIG`).
//! Приоритет: env > файл > умолчание.
//!
//! Разбор чистый: источники значений (env, чтение файла) приходят замыканиями, поэтому
//! каждое правило проверяется без обращения к настоящему окружению. Файл (`file`) лишь
//! достраивает поиск по имени переменной, разбор отдельных параметров о нём не знает.

mod network;
mod proxy;
mod repos;
mod source;
mod storage;
mod token;

use std::io;
use std::path::Path;

use crane_domain::access::Principal;
use crane_http::TrustedProxies;
use source::{Secret, secret};

pub(crate) use network::admin_port;
pub(crate) use proxy::{Cache, Proxy};
pub(crate) use repos::{Repos, proxy_var};
pub(crate) use source::layered;
pub(crate) use storage::{Database, Storage};

/// Значение «в памяти» для хранилища, blob и кэша.
const MEMORY: &str = "memory";

/// Имена всех переменных; из них выводятся ключи файла (`source::file`). Каждая секция владеет
/// своим списком, здесь они только сводятся. `CRANE_CONFIG` сюда не входит намеренно.
fn vars() -> impl Iterator<Item = &'static str> {
    [
        network::VARS.as_slice(),
        token::VARS.as_slice(),
        storage::VARS.as_slice(),
        proxy::VARS.as_slice(),
        repos::VARS.as_slice(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .chain(repos::list_vars())
}

/// Ошибка конфигурации. Значения секретных переменных в тексте не появляются никогда.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("не задана переменная {var}")]
    Missing { var: &'static str },
    #[error("некорректное значение {var}: {reason}")]
    Invalid {
        var: &'static str,
        reason: &'static str,
    },
    #[error("переменные {first} и {second} взаимоисключающие")]
    Conflict {
        first: &'static str,
        second: &'static str,
    },
    /// В `scheme` только схема URL, без логина, пароля и адреса.
    #[error("{var}: значение со схемой `{scheme}` пока не поддерживается")]
    Unsupported { var: &'static str, scheme: String },
    /// `var` — имя переменной `*_FILE`; причина ввода-вывода не содержит содержимого файла.
    #[error("не удалось прочитать {var}: {reason}")]
    SecretFile { var: &'static str, reason: String },
    /// Ключ запроса `CRANE_BLOBS` вне списка несекретных параметров; значение в ошибку не попадает.
    #[error(
        "{}: ключ `{key}` не из списка несекретных параметров; учётные данные и их \
         поставщики S3 задаются только переменными AWS_*",
        storage::BLOBS
    )]
    BlobsKey { key: String },
    /// Каталог в PostgreSQL над blob в памяти после рестарта ссылался бы на потерянные blob.
    #[error(
        "{} и {}: memory нельзя смешивать с постоянным хранилищем",
        storage::DATABASE_URL,
        storage::BLOBS
    )]
    MixedStorage,
    /// Ключ S3 из файлов при `memory`: мёртвая конфигурация хуже ошибки.
    #[error(
        "{} и {}: ключ S3 не нужен при memory",
        storage::S3_KEY_ID_FILE,
        storage::S3_SECRET_FILE
    )]
    S3KeyWithMemory,
    #[error("неизвестный ключ `{key}` в файле конфигурации")]
    UnknownKey { key: String },
    #[error("некорректное значение ключа `{key}` в файле конфигурации: {reason}")]
    FileValue { key: String, reason: &'static str },
    #[error("не удалось прочитать файл конфигурации {path}: {reason}")]
    FileRead { path: String, reason: String },
    /// Только позиция: сообщение парсера цитирует строку файла, а там может быть токен.
    #[error("ошибка разбора TOML в {path}: строка {line}, столбец {column}")]
    FileParse {
        path: String,
        line: usize,
        column: usize,
    },
}

pub(crate) struct Settings {
    /// Кому принадлежит статический токен: попадает в аудит и в ответ `owners`.
    pub(crate) principal: Principal,
    pub(crate) trusted_proxies: TrustedProxies,
    pub(crate) http_port: u16,
    pub(crate) admin_port: u16,
    pub(crate) public_url: String,
    pub(crate) token: Secret,
    /// Hosted- и proxy-репозитории по форматам.
    pub(crate) repos: Repos,
    pub(crate) max_publish_bytes: usize,
    pub(crate) storage: Storage,
    pub(crate) proxy: Proxy,
}

impl Settings {
    /// Порядок вычисления полей определяет, какая из неверных переменных будет названа первой:
    /// репозитории, кэш, предел proxy, затем остальное, хранилище последним.
    pub(crate) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        let repos = Repos::from_env(var)?;
        let proxy = Proxy::from_env(var, read_file)?;
        Ok(Self {
            principal: token::principal(var)?,
            trusted_proxies: network::trusted_proxies(var)?,
            http_port: network::http_port(var)?,
            admin_port: admin_port(var)?,
            public_url: network::public_url(var)?,
            token: secret(var, read_file, token::TOKEN, token::TOKEN_FILE)?,
            repos,
            max_publish_bytes: repos::max_publish_bytes(var)?,
            storage: Storage::from_env(var, read_file)?,
            proxy,
        })
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
