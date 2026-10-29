//! Статический токен и его владелец.

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::settings::{Kind, SettingDef};

use super::source::{failures, secret, setting};
use super::{ConfigError, Read, Section, Var};

pub(super) const TOKEN: &str = "CRANE_TOKEN";
pub(super) const TOKEN_FILE: &str = "CRANE_TOKEN_FILE";
const TOKEN_PRINCIPAL: &str = "CRANE_TOKEN_PRINCIPAL";
/// Совпадает с прежним логином владельца в ответе cargo `owners`.
const DEFAULT_PRINCIPAL: &str = "crane";

const GROUP: &str = "token";
const PRINCIPAL_MAX_LEN: usize = 64;

pub(super) const SECTION: Section = Section {
    group: GROUP,
    vars,
    settings,
    check,
};

fn vars() -> Vec<&'static str> {
    vec![TOKEN, TOKEN_FILE, TOKEN_PRINCIPAL]
}

fn settings() -> Vec<SettingDef> {
    vec![
        setting(GROUP, "token", TOKEN, Kind::Secret),
        setting(GROUP, "token_principal", TOKEN_PRINCIPAL, Kind::Text),
    ]
}

fn check(var: Var, read_file: Read, _: bool) -> Vec<ConfigError> {
    failures([
        principal(&var).err(),
        secret(&var, &read_file, TOKEN, TOKEN_FILE).err(),
    ])
}

pub(super) fn principal(var: &impl Fn(&str) -> Option<String>) -> Result<Principal, ConfigError> {
    let id = var(TOKEN_PRINCIPAL).unwrap_or_else(|| DEFAULT_PRINCIPAL.to_owned());
    let id = id.trim();
    // Идентификатор попадает в каждое событие аудита и в access-журнал. Двоеточие грамматикой
    // не допускается, поэтому служебные `crane:system`, `crane:bootstrap` и всё под `cli:` недостижимы:
    // токен не может писать в журнал от имени самого CRANE или его CLI.
    if !is_valid_id(id) {
        return Err(ConfigError::Invalid {
            var: TOKEN_PRINCIPAL,
            reason: "ожидалось имя вида [a-z0-9][a-z0-9._-]{0,63}",
        });
    }
    Ok(Principal {
        id: id.to_owned(),
        kind: PrincipalKind::Robot,
    })
}

/// `^[a-z0-9][a-z0-9._-]{0,63}$`.
fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && id.len() <= PRINCIPAL_MAX_LEN
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
