//! Конфигурация: переменные окружения поверх необязательного файла TOML (`CRANE_CONFIG`).
//! Приоритет: env > файл > умолчание.
//!
//! Разбор чистый: источники значений (env, чтение файла) приходят замыканиями, поэтому
//! каждое правило проверяется без обращения к настоящему окружению. Файл (`file`) лишь
//! достраивает поиск по имени переменной, разбор отдельных параметров о нём не знает.

mod namespaces;
mod network;
mod proxy;
mod repos;
mod source;
mod storage;
mod token;

use std::io;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crane_domain::access::Principal;
use crane_domain::registry::Namespace;
use crane_domain::settings::{Secret, SettingDef};
use crane_http::TrustedProxies;
use crane_pg::KeyError;
use source::secret;

pub(crate) use network::admin_port;
pub(crate) use proxy::{Cache, Proxy};
pub(crate) use repos::{Repos, proxy_var};
pub(crate) use source::{ConfigSchema, StoredLayer, layered};
pub(crate) use storage::{Blobs, Database, settings_key};

/// Поиск значения по имени переменной в слое env + файл; разделяется со схемой настроек.
pub(crate) type Lookup = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;
/// Чтение файла (секреты, `CRANE_CONFIG`); разделяется со схемой настроек.
pub(crate) type ReadFile = Arc<dyn Fn(&Path) -> io::Result<String> + Send + Sync>;

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
        namespaces::VARS.as_slice(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .chain(repos::list_vars())
}

/// Определения всех ключей настроек. Ключ списка репозиториев строится из `REPO_VARS`, поэтому
/// таблица собирается один раз на процесс (см. `repos::list_settings`).
pub(crate) fn definitions() -> &'static [SettingDef] {
    static ALL: OnceLock<Vec<SettingDef>> = OnceLock::new();
    ALL.get_or_init(|| {
        [
            network::SETTINGS.as_slice(),
            token::SETTINGS.as_slice(),
            storage::SETTINGS.as_slice(),
            proxy::SETTINGS.as_slice(),
            repos::SETTINGS.as_slice(),
            namespaces::SETTINGS.as_slice(),
        ]
        .into_iter()
        .flatten()
        .copied()
        .chain(repos::list_settings())
        .collect()
    })
}

pub(crate) fn definition(key: &str) -> Option<&'static SettingDef> {
    definitions().iter().find(|def| def.key == key)
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
    /// Строка секрета в БД не открывается: причина называет только отпечатки ключей, без значений.
    #[error("настройка `{key}` в БД не читается: {reason}")]
    Unreadable { key: String, reason: String },
    /// `KeyError` называет только длину ключа, содержимое не цитирует.
    #[error("{}: {reason}", storage::SETTINGS_KEY)]
    SettingsKey { reason: KeyError },
    /// Ключ шифрования при `memory` не нужен: мёртвая конфигурация хуже ошибки.
    #[error("{}: ключ шифрования не нужен при memory", storage::SETTINGS_KEY)]
    SettingsKeyWithMemory,
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

impl ConfigError {
    /// Переменная, из-за которой разбор отказал; по ней схема находит ключ настройки.
    /// `None`, когда ошибка относится к файлу или к строке БД, а не к переменной.
    pub(crate) fn var(&self) -> Option<&'static str> {
        match self {
            Self::Missing { var }
            | Self::Invalid { var, .. }
            | Self::Unsupported { var, .. }
            | Self::SecretFile { var, .. } => Some(var),
            Self::Conflict { first, .. } => Some(first),
            Self::MixedStorage | Self::S3KeyWithMemory | Self::BlobsKey { .. } => {
                Some(storage::BLOBS)
            }
            Self::SettingsKey { .. } | Self::SettingsKeyWithMemory => Some(storage::SETTINGS_KEY),
            Self::Unreadable { .. }
            | Self::UnknownKey { .. }
            | Self::FileValue { .. }
            | Self::FileRead { .. }
            | Self::FileParse { .. } => None,
        }
    }
}

pub(crate) struct Config {
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
    /// Blob-хранилище; каталог (`Database`) живёт отдельно: пароль БД в `Config` не попадает.
    pub(crate) blobs: Blobs,
    pub(crate) proxy: Proxy,
    /// Резервирования платформы (`CRANE_NAMESPACES`), применяются при старте.
    pub(crate) namespaces: Vec<Namespace>,
}

impl Config {
    /// Конфигурация над слоями env + файл (`inherited`) и БД (`stored`). Приоритет решает
    /// `StoredLayer` по ключу, здесь слои лишь склеиваются в один поиск по имени переменной.
    pub(crate) fn from_layers(
        inherited: &impl Fn(&str) -> Option<String>,
        stored: &StoredLayer,
        read_file: &impl Fn(&Path) -> io::Result<String>,
        persistent: bool,
    ) -> Result<Self, ConfigError> {
        Self::from_env(
            &|name: &str| inherited(name).or_else(|| stored.get(name)),
            read_file,
            persistent,
        )
    }

    /// Порядок вычисления полей определяет, какая из неверных переменных будет названа первой:
    /// репозитории, кэш, предел proxy, затем остальное, хранилище последним.
    /// `persistent` — режим каталога, решённый один раз в `Database`: `Config` его не выводит.
    pub(crate) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
        persistent: bool,
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
            blobs: Blobs::from_env(var, read_file, persistent)?,
            proxy,
            namespaces: namespaces::from_env(var)?,
        })
    }
}

impl Config {
    /// Прогон одной секции без остальных: первая ошибка чужой секции не должна заслонять свою.
    /// Секции со связанными инвариантами (`storage`) получают режим каталога отдельно.
    pub(crate) fn check_section(
        group: &str,
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
        persistent: bool,
    ) -> Vec<ConfigError> {
        let errors = match group {
            network::GROUP => vec![
                network::http_port(var).err(),
                admin_port(var).err(),
                network::public_url(var).err(),
                network::trusted_proxies(var).err(),
            ],
            token::GROUP => vec![
                token::principal(var).err(),
                secret(var, read_file, token::TOKEN, token::TOKEN_FILE).err(),
            ],
            storage::GROUP => vec![Blobs::from_env(var, read_file, persistent).err()],
            proxy::GROUP => vec![Proxy::from_env(var, read_file).err()],
            repos::GROUP => vec![
                Repos::from_env(var).err(),
                repos::max_publish_bytes(var).err(),
            ],
            namespaces::GROUP => vec![namespaces::from_env(var).err()],
            _ => Vec::new(),
        };
        errors.into_iter().flatten().collect()
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
