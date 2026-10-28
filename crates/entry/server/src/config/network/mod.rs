//! Сетевые параметры: порты, публичный адрес, доверенные прокси.

use crane_domain::settings::{Kind, SettingDef};
use crane_http::TrustedProxies;

use super::source::{failures, number, required, setting};
use super::{ConfigError, Read, Section, Var};

pub(super) const HTTP_PORT: &str = "CRANE_HTTP_PORT";
pub(super) const ADMIN_PORT: &str = "CRANE_ADMIN_PORT";
pub(super) const PUBLIC_URL: &str = "CRANE_PUBLIC_URL";
const TRUSTED_PROXIES: &str = "CRANE_TRUSTED_PROXIES";
const DEFAULT_HTTP_PORT: u16 = 8080;
const DEFAULT_ADMIN_PORT: u16 = 9464;

const GROUP: &str = "network";

pub(super) const SECTION: Section = Section {
    group: GROUP,
    vars,
    settings,
    check,
};

fn vars() -> Vec<&'static str> {
    vec![HTTP_PORT, ADMIN_PORT, PUBLIC_URL, TRUSTED_PROXIES]
}

/// Порты и доверенные прокси меняют только окружение или файл: от них зависит, достижима ли консоль.
fn settings() -> Vec<SettingDef> {
    vec![
        SettingDef {
            console: false,
            ..setting(GROUP, "http_port", HTTP_PORT, Kind::Number)
        },
        SettingDef {
            console: false,
            ..setting(GROUP, "admin_port", ADMIN_PORT, Kind::Number)
        },
        setting(GROUP, "public_url", PUBLIC_URL, Kind::Url),
        SettingDef {
            console: false,
            ..setting(GROUP, "trusted_proxies", TRUSTED_PROXIES, Kind::List)
        },
    ]
}

fn check(var: Var, _: Read, _: bool) -> Vec<ConfigError> {
    failures([
        http_port(&var).err(),
        admin_port(&var).err(),
        public_url(&var).err(),
        trusted_proxies(&var).err(),
    ])
}

pub(super) fn http_port(var: &impl Fn(&str) -> Option<String>) -> Result<u16, ConfigError> {
    number(var, HTTP_PORT, DEFAULT_HTTP_PORT)
}

/// Отдельно от `Config`: проба здоровья не должна требовать токен, адрес и хранилище.
pub(crate) fn admin_port(var: &impl Fn(&str) -> Option<String>) -> Result<u16, ConfigError> {
    number(var, ADMIN_PORT, DEFAULT_ADMIN_PORT)
}

pub(super) fn public_url(var: &impl Fn(&str) -> Option<String>) -> Result<String, ConfigError> {
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

pub(super) fn trusted_proxies(
    var: &impl Fn(&str) -> Option<String>,
) -> Result<TrustedProxies, ConfigError> {
    TrustedProxies::parse(&var(TRUSTED_PROXIES).unwrap_or_default()).map_err(|_| {
        ConfigError::Invalid {
            var: TRUSTED_PROXIES,
            reason: "ожидался список IP-адресов или CIDR через запятую",
        }
    })
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
