//! Статический токен и его владелец.

use std::io;
use std::path::Path;

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::settings::{Kind, Secret, SettingDef};

use super::source::{failures, secret, setting};
use super::{ConfigError, Read, Section, Var};

pub(super) const TOKEN: &str = "CRANE_TOKEN";
pub(super) const TOKEN_FILE: &str = "CRANE_TOKEN_FILE";
const BOOTSTRAP_TOKEN: &str = "CRANE_BOOTSTRAP_TOKEN";
const BOOTSTRAP_TOKEN_FILE: &str = "CRANE_BOOTSTRAP_TOKEN_FILE";
/// Префикс, по которому сервер отличает bootstrap-токен от статического (20.5).
pub(crate) const BOOTSTRAP_PREFIX: &str = "crb_";
/// Заданный оператором токен короче этого - не секрет, а подсказка для подбора (256 бит у выпущенного).
const BOOTSTRAP_MIN_LEN: usize = 36;
const TOKEN_PRINCIPAL: &str = "CRANE_TOKEN_PRINCIPAL";
/// Совпадает с прежним логином владельца в ответе cargo `owners`.
const DEFAULT_PRINCIPAL: &str = "crane";

const GROUP: &str = "token";

pub(super) const SECTION: Section = Section {
    group: GROUP,
    vars,
    settings,
    check,
};

fn vars() -> Vec<&'static str> {
    vec![
        TOKEN,
        TOKEN_FILE,
        TOKEN_PRINCIPAL,
        BOOTSTRAP_TOKEN,
        BOOTSTRAP_TOKEN_FILE,
    ]
}

fn settings() -> Vec<SettingDef> {
    vec![
        setting(GROUP, "token", TOKEN, Kind::Secret),
        setting(GROUP, "token_principal", TOKEN_PRINCIPAL, Kind::Text),
        // Первичный секрет читается до слоя БД и открывает мастер: из консоли его не менять.
        SettingDef {
            console: false,
            ..setting(GROUP, "bootstrap_token", BOOTSTRAP_TOKEN, Kind::Secret)
        },
    ]
}

fn check(var: Var, read_file: Read, _: bool) -> Vec<ConfigError> {
    failures([
        principal(&var).err(),
        secret(&var, &read_file, TOKEN, TOKEN_FILE).err(),
    ])
}

/// Задан ли статический токен окружением (переменной или файлом). Хранимый в БД не считается:
/// на первом старте строк `setting` ещё нет, а нечитаемую строку решает мастер.
pub(crate) fn static_token_set(var: &impl Fn(&str) -> Option<String>) -> bool {
    var(TOKEN).is_some() || var(TOKEN_FILE).is_some()
}

/// Bootstrap-токен, заданный оператором (`CRANE_BOOTSTRAP_TOKEN` или `_FILE`). `None`, если не задан.
/// Без префикса `crb_` сервер принял бы его за статический, поэтому такой токен отвергается.
pub(crate) fn bootstrap_token(
    var: &impl Fn(&str) -> Option<String>,
    read_file: &impl Fn(&Path) -> io::Result<String>,
) -> Result<Option<Secret>, ConfigError> {
    if var(BOOTSTRAP_TOKEN).is_none() && var(BOOTSTRAP_TOKEN_FILE).is_none() {
        return Ok(None);
    }
    let token = secret(var, read_file, BOOTSTRAP_TOKEN, BOOTSTRAP_TOKEN_FILE)?;
    let value = token.expose();
    if !value.starts_with(BOOTSTRAP_PREFIX) || value.len() < BOOTSTRAP_MIN_LEN {
        return Err(ConfigError::Invalid {
            var: BOOTSTRAP_TOKEN,
            reason: "ожидался токен вида crb_… не короче 36 символов",
        });
    }
    Ok(Some(token))
}

pub(super) fn principal(var: &impl Fn(&str) -> Option<String>) -> Result<Principal, ConfigError> {
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

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
