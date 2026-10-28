//! Репозитории формата: списки hosted и proxy и предел размера публикации.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crane_domain::registry::{Format, RepositoryKind};
use crane_domain::settings::{Kind, SettingDef, Unit};

use super::ConfigError;
use super::source::{key_of, number, setting};

const CARGO_REPOS: &str = "CRANE_CARGO_REPOS";
const CARGO_PROXY_REPOS: &str = "CRANE_CARGO_PROXY_REPOS";
const NPM_REPOS: &str = "CRANE_NPM_REPOS";
const NPM_PROXY_REPOS: &str = "CRANE_NPM_PROXY_REPOS";
pub(super) const MAX_PUBLISH_BYTES: &str = "CRANE_MAX_PUBLISH_BYTES";
const DEFAULT_REPOS: &str = "internal";
const DEFAULT_MAX_PUBLISH_BYTES: usize = 10 * 1024 * 1024;

/// Имена переменных без списков: те выводятся из `REPO_VARS` (`list_vars`).
pub(super) const VARS: [&str; 1] = [MAX_PUBLISH_BYTES];

pub(super) const GROUP: &str = "repos";

/// Списки репозиториев здесь не перечислены: их определения строит `list_settings`.
pub(super) const SETTINGS: [SettingDef; 1] = [SettingDef {
    unit: Some(Unit::Bytes),
    ..setting(GROUP, "max_publish_bytes", MAX_PUBLISH_BYTES, Kind::Number)
}];

/// Определения списков всех форматов, выведенные из `REPO_VARS`: новый формат ключ не дописывает.
/// Имена ключей строятся из имён переменных один раз и живут в статике, поэтому вызывать можно
/// сколько угодно.
pub(super) fn list_settings() -> impl Iterator<Item = SettingDef> {
    static KEYS: OnceLock<Vec<String>> = OnceLock::new();
    let keys = KEYS.get_or_init(|| list_vars().map(key_of).collect());
    list_vars().zip(keys).map(|(var, key)| SettingDef {
        key: key.as_str(),
        ..setting(GROUP, "", var, Kind::List)
    })
}

/// Имена переменных списков всех форматов. Новый формат сюда не дописывается: достаточно `REPO_VARS`.
pub(super) fn list_vars() -> impl Iterator<Item = &'static str> {
    REPO_VARS.iter().flat_map(|vars| [vars.hosted, vars.proxy])
}

/// Переменные списков репозиториев формата. Новый формат — одна строка здесь и ни одной новой функции.
struct RepoVars {
    format: Format,
    hosted: &'static str,
    proxy: &'static str,
    /// Hosted-репозиторий по умолчанию; `None` — по умолчанию репозиториев нет.
    hosted_default: Option<&'static str>,
}

const REPO_VARS: [RepoVars; 2] = [
    RepoVars {
        format: Format::Cargo,
        hosted: CARGO_REPOS,
        proxy: CARGO_PROXY_REPOS,
        hosted_default: Some(DEFAULT_REPOS),
    },
    RepoVars {
        format: Format::Npm,
        hosted: NPM_REPOS,
        proxy: NPM_PROXY_REPOS,
        hosted_default: None,
    },
];

/// Имя переменной со списком proxy-репозиториев формата (для сообщений об ошибках).
pub(crate) fn proxy_var(format: Format) -> &'static str {
    REPO_VARS
        .iter()
        .find(|vars| vars.format == format)
        .map_or("CRANE_<FORMAT>_PROXY_REPOS", |vars| vars.proxy)
}

/// Репозитории из конфигурации по форматам: что завести в каталоге при старте.
/// Имя уникально внутри формата для обоих видов: это ключ каталога `(format, name)`.
#[derive(Debug)]
pub(crate) struct Repos(BTreeMap<Format, Lists>);

#[derive(Debug, Default)]
struct Lists {
    hosted: Vec<String>,
    proxy: Vec<String>,
}

impl Lists {
    fn of(&self, kind: RepositoryKind) -> &[String] {
        match kind {
            RepositoryKind::Hosted => &self.hosted,
            RepositoryKind::Proxy => &self.proxy,
        }
    }
}

impl Repos {
    /// Hosted- и proxy-списки всех форматов. Порядок разбора: формат за форматом, в нём hosted, затем
    /// proxy; от него зависит, какая из неверных переменных будет названа первой.
    pub(super) fn from_env(var: &impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let mut formats = BTreeMap::new();
        for vars in &REPO_VARS {
            let hosted = hosted_list(var, vars)?;
            let proxy = proxy_list(var, vars, &hosted)?;
            formats.insert(vars.format, Lists { hosted, proxy });
        }
        Ok(Self(formats))
    }

    #[cfg(test)]
    pub(crate) fn of(format: Format, kind: RepositoryKind, names: Vec<String>) -> Self {
        let mut lists = Lists::default();
        match kind {
            RepositoryKind::Hosted => lists.hosted = names,
            RepositoryKind::Proxy => lists.proxy = names,
        }
        Self(BTreeMap::from([(format, lists)]))
    }

    pub(crate) fn names(&self, format: Format, kind: RepositoryKind) -> &[String] {
        self.0.get(&format).map_or(&[], |lists| lists.of(kind))
    }

    pub(crate) fn of_kind(
        &self,
        kind: RepositoryKind,
    ) -> impl Iterator<Item = (Format, &[String])> {
        self.0
            .iter()
            .map(move |(format, lists)| (*format, lists.of(kind)))
    }

    /// Есть ли хоть один репозиторий вида `kind` в любом формате.
    pub(crate) fn any(&self, kind: RepositoryKind) -> bool {
        self.0.values().any(|lists| !lists.of(kind).is_empty())
    }
}

pub(super) fn max_publish_bytes(
    var: &impl Fn(&str) -> Option<String>,
) -> Result<usize, ConfigError> {
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

/// Имя попадает в путь `/<формат>/<имя>`, поэтому набор символов ограничен: `{`, `}` и `/` сломали бы маршрутизацию.
fn repo_name_is_valid(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Разбирает список имён: непустые, допустимые, без повторов; `taken` — имена, которые занимать нельзя.
fn names(var: &'static str, raw: &str, taken: &[String]) -> Result<Vec<String>, ConfigError> {
    let mut names: Vec<String> = Vec::new();
    for item in raw.split(',') {
        let name = item.trim();
        let reason = if name.is_empty() {
            Some("пустое имя репозитория в списке")
        } else if !repo_name_is_valid(name) {
            Some("имя может содержать только буквы, цифры, `-`, `_` и `.`")
        } else if taken.iter().any(|other| other == name) {
            Some("имя совпадает с hosted-репозиторием")
        } else if names.iter().any(|seen| seen == name) {
            Some("имя повторяется в списке")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(ConfigError::Invalid { var, reason });
        }
        names.push(name.to_owned());
    }
    Ok(names)
}

/// Без значения действует умолчание формата; у форматов без умолчания пустое значение — пустой список.
fn hosted_list(
    var: &impl Fn(&str) -> Option<String>,
    vars: &RepoVars,
) -> Result<Vec<String>, ConfigError> {
    let raw = match (var(vars.hosted), vars.hosted_default) {
        (Some(raw), Some(_)) => raw,
        (Some(raw), None) if !raw.trim().is_empty() => raw,
        (None, Some(default)) => default.to_owned(),
        (Some(_) | None, None) => return Ok(Vec::new()),
    };
    names(vars.hosted, &raw, &[])
}

/// Пустая переменная (compose передаёт `""`) и отсутствующая — пустой список.
fn proxy_list(
    var: &impl Fn(&str) -> Option<String>,
    vars: &RepoVars,
    hosted: &[String],
) -> Result<Vec<String>, ConfigError> {
    let raw = var(vars.proxy).unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    names(vars.proxy, &raw, hosted)
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
