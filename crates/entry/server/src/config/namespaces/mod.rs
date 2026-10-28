//! `CRANE_NAMESPACES`: резервирование имён платформой при старте (10.3, 16.1).

use std::str::FromStr;

use crane_domain::registry::{Format, Namespace, NamespacePattern, WorkspaceId};
use crane_domain::settings::{Kind, SettingDef};
use uuid::Uuid;

use super::source::{failures, setting};
use super::{ConfigError, Read, Section, Var};
use crate::ecosystem::ecosystem;

pub(super) const NAMESPACES: &str = "CRANE_NAMESPACES";

const GROUP: &str = "namespaces";

pub(super) const SECTION: Section = Section {
    group: GROUP,
    vars,
    settings,
    check,
};

fn vars() -> Vec<&'static str> {
    vec![NAMESPACES]
}

fn settings() -> Vec<SettingDef> {
    vec![setting(GROUP, "namespaces", NAMESPACES, Kind::List)]
}

fn check(var: Var, _: Read, _: bool) -> Vec<ConfigError> {
    failures([from_env(&var).err()])
}

fn invalid(reason: &'static str) -> ConfigError {
    ConfigError::Invalid {
        var: NAMESPACES,
        reason,
    }
}

pub(super) fn from_env(
    var: &impl Fn(&str) -> Option<String>,
) -> Result<Vec<Namespace>, ConfigError> {
    parse(&var(NAMESPACES).unwrap_or_default())
}

/// Список `формат:шаблон` или `формат:шаблон=uuid-workspace` через запятую.
/// Пустая и отсутствующая переменная — пустой список. Значение в ошибку не попадает целиком.
pub(super) fn parse(raw: &str) -> Result<Vec<Namespace>, ConfigError> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<Namespace> = Vec::new();
    for item in raw.split(',') {
        let entry = entry(item.trim())?;
        // Совпадение после нормализации: `Acme-*` и `acme_*` в cargo один и тот же шаблон.
        if entries
            .iter()
            .any(|seen| seen.format == entry.format && seen.pattern == entry.pattern)
        {
            return Err(invalid("шаблон повторяется в списке"));
        }
        entries.push(entry);
    }
    Ok(entries)
}

fn entry(item: &str) -> Result<Namespace, ConfigError> {
    if item.is_empty() {
        return Err(invalid("пустая запись в списке"));
    }
    // Владелец после последнего `=`: в шаблонах формата `=` не бывает, а uuid его не содержит.
    let (spec, workspace) = match item.rsplit_once('=') {
        Some((spec, uuid)) => {
            let uuid =
                Uuid::parse_str(uuid.trim()).map_err(|_| invalid("некорректный uuid workspace"))?;
            (spec, WorkspaceId::new(uuid))
        }
        None => (item, WorkspaceId::DEFAULT),
    };
    // Формат до первого `:`: в шаблоне Maven двоеточие есть (`groupId:artifactId`).
    let (format, pattern) = spec
        .split_once(':')
        .ok_or_else(|| invalid("ожидается `формат:шаблон`"))?;
    let format = Format::from_str(format.trim()).map_err(|_| invalid("неизвестный формат"))?;
    let pattern = NamespacePattern::parse(ecosystem(format), pattern.trim())
        .map_err(|_| invalid("некорректный шаблон"))?;
    Ok(Namespace {
        format,
        pattern,
        workspace,
    })
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
