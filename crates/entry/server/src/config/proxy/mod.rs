//! Настройки proxy-движка: кэш и предел размера проксируемого артефакта.

mod cache;

use std::io;
use std::path::Path;

use crane_app::proxy::ProxyLimits;
use crane_domain::settings::{Kind, SettingDef, Unit};

pub(crate) use cache::Cache;

use super::source::{failures, number, setting};
use super::{ConfigError, Read, Section, Var};

const CACHE: &str = "CRANE_CACHE";
const CACHE_FILE: &str = "CRANE_CACHE_FILE";
const PROXY_MAX_ARTIFACT_BYTES: &str = "CRANE_PROXY_MAX_ARTIFACT_BYTES";

const GROUP: &str = "proxy";

pub(super) const SECTION: Section = Section {
    group: GROUP,
    vars,
    settings,
    check,
};

fn vars() -> Vec<&'static str> {
    vec![CACHE, CACHE_FILE, PROXY_MAX_ARTIFACT_BYTES]
}

fn settings() -> Vec<SettingDef> {
    vec![
        setting(GROUP, "cache", CACHE, Kind::Secret),
        SettingDef {
            unit: Some(Unit::Bytes),
            ..setting(
                GROUP,
                "proxy_max_artifact_bytes",
                PROXY_MAX_ARTIFACT_BYTES,
                Kind::Number,
            )
        },
    ]
}

fn check(var: Var, read_file: Read, _: bool) -> Vec<ConfigError> {
    failures([Proxy::from_env(&var, &read_file).err()])
}

/// Настройки proxy-движка. Без `Debug`, как `Cache`: в адресе Valkey бывает пароль.
pub(crate) struct Proxy {
    pub(crate) cache: Cache,
    /// Потолок размера одного проксируемого артефакта в байтах.
    pub(crate) max_artifact_bytes: u64,
}

impl Proxy {
    pub(super) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        Ok(Self {
            cache: Cache::from_env(var, read_file)?,
            max_artifact_bytes: proxy_max_artifact_bytes(var)?,
        })
    }
}

fn proxy_max_artifact_bytes(var: &impl Fn(&str) -> Option<String>) -> Result<u64, ConfigError> {
    let limit = number(
        var,
        PROXY_MAX_ARTIFACT_BYTES,
        ProxyLimits::DEFAULT.max_artifact_bytes,
    )?;
    // Нулевой предел отклонял бы любой архив.
    if limit == 0 {
        return Err(ConfigError::Invalid {
            var: PROXY_MAX_ARTIFACT_BYTES,
            reason: "предел должен быть больше нуля",
        });
    }
    Ok(limit)
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
