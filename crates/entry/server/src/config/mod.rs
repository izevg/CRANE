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

type VarFn = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;
type ReadFn = Arc<dyn Fn(&Path) -> io::Result<String> + Send + Sync>;

/// Слой, который не меняется за жизнь процесса: окружение с файлом TOML и чтение файлов секретов.
/// Один раз разбирается при старте и разделяется со схемой настроек; его же перечитывает
/// цикл двухфазного serve (B3b), файл TOML при этом не читается заново.
#[derive(Clone)]
pub(crate) struct Boot {
    var: VarFn,
    read_file: ReadFn,
}

impl Boot {
    pub(crate) fn new(
        var: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
        read_file: impl Fn(&Path) -> io::Result<String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            var: Arc::new(var),
            read_file: Arc::new(read_file),
        }
    }

    /// Значение переменной по имени, в том числе `AWS_*_FILE`, которых нет в `vars()`.
    pub(crate) fn var(&self, name: &str) -> Option<String> {
        (self.var)(name)
    }

    pub(crate) fn read(&self, path: &Path) -> io::Result<String> {
        (self.read_file)(path)
    }
}

/// Значение «в памяти» для хранилища, blob и кэша.
const MEMORY: &str = "memory";

/// Значение переменной по имени: слой env + файл или его наложение со слоем БД.
pub(crate) type Var<'a> = &'a dyn Fn(&str) -> Option<String>;
/// Чтение файла секрета или конфигурации.
pub(crate) type Read<'a> = &'a dyn Fn(&Path) -> io::Result<String>;

/// Секция конфигурации: её переменные, ключи консоли и проверка отдельно от остальных секций.
/// Группа ключей берётся отсюда же, поэтому определение без проверки непредставимо.
pub(crate) struct Section {
    group: &'static str,
    /// Все переменные секции, включая `*_FILE` секретов; из них выводятся ключи файла TOML.
    vars: fn() -> Vec<&'static str>,
    settings: fn() -> Vec<SettingDef>,
    /// Все ошибки секции, а не первая: консоль показывает их разом. Третий аргумент — режим
    /// каталога (`Database::persistent`), он нужен связи blob с каталогом.
    check: fn(Var, Read, bool) -> Vec<ConfigError>,
}

impl Section {
    pub(crate) fn check(&self, var: Var, read_file: Read, persistent: bool) -> Vec<ConfigError> {
        (self.check)(var, read_file, persistent)
    }
}

/// Все секции. Новая секция — одна строка здесь.
const SECTIONS: [Section; 6] = [
    network::SECTION,
    token::SECTION,
    storage::SECTION,
    proxy::SECTION,
    repos::SECTION,
    namespaces::SECTION,
];

/// Секция по имени группы ключей.
pub(crate) fn section(group: &str) -> Option<&'static Section> {
    SECTIONS.iter().find(|section| section.group == group)
}

/// Имена всех переменных; из них выводятся ключи файла (`source::file`).
/// `CRANE_CONFIG` сюда не входит намеренно.
fn vars() -> impl Iterator<Item = &'static str> {
    SECTIONS.iter().flat_map(|section| (section.vars)())
}

/// Определения всех ключей настроек, собранные один раз на процесс.
pub(crate) fn definitions() -> &'static [SettingDef] {
    static ALL: OnceLock<Vec<SettingDef>> = OnceLock::new();
    ALL.get_or_init(|| {
        SECTIONS
            .iter()
            .flat_map(|section| (section.settings)())
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

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
