use std::fmt;
use std::hash::{Hash, Hasher};

use super::Format;

const CARGO_MAX_LEN: usize = 64;
/// Предел npm в байтах всего имени, включая `@scope/`.
const NPM_MAX_LEN: usize = 214;
/// Имена, которые npm держит за собой (`validate-npm-package-name`).
const NPM_RESERVED: [&str; 2] = ["node_modules", "favicon.ico"];

/// Ошибка разбора имени пакета.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("имя пустое")]
    Empty,
    #[error("имя длиннее допустимого: {len} > {max}")]
    TooLong { len: usize, max: usize },
    #[error("недопустимый символ {0:?}")]
    InvalidChar(char),
    #[error("имя не может начинаться с {0:?}")]
    InvalidStart(char),
    #[error("имя зарезервировано: {0:?}")]
    Reserved(String),
    #[error("некорректная форма scope: ожидается «@scope/name» с непустыми частями")]
    MalformedScope,
    #[error("формат {0:?} пока не поддержан")]
    Unsupported(Format),
}

/// Имя пакета: равенство и хеш считаются по нормализованной форме,
/// а исходное написание сохраняется для отдачи клиентам.
#[derive(Debug, Clone)]
pub struct PackageName {
    normalized: String,
    written: String,
}

impl PackageName {
    pub fn parse(format: Format, raw: &str) -> Result<Self, NameError> {
        match format {
            Format::Cargo => Self::parse_cargo(raw),
            Format::Npm => Self::parse_npm(raw),
            Format::Maven | Format::PyPi | Format::NuGet => Err(NameError::Unsupported(format)),
        }
    }

    /// Грамматика npm, допустимая для того, что может отдавать upstream (в том числе старые имена
    /// с заглавными буквами вроде `JSONStream`). Строже для новых пакетов — политика формата, не грамматика.
    /// Регистр сохраняется и в нормализованной форме: имена npm регистрозависимы.
    fn parse_npm(raw: &str) -> Result<Self, NameError> {
        if raw.is_empty() {
            return Err(NameError::Empty);
        }
        if raw.len() > NPM_MAX_LEN {
            return Err(NameError::TooLong {
                len: raw.len(),
                max: NPM_MAX_LEN,
            });
        }
        if NPM_RESERVED.contains(&raw) {
            return Err(NameError::Reserved(raw.to_owned()));
        }
        match raw.strip_prefix('@') {
            Some(scoped) => {
                let (scope, name) = scoped.split_once('/').ok_or(NameError::MalformedScope)?;
                if scope.is_empty() || name.is_empty() {
                    return Err(NameError::MalformedScope);
                }
                check_npm_part(scope)?;
                check_npm_part(name)?;
            }
            None => check_npm_part(raw)?,
        }
        Ok(Self {
            normalized: raw.to_owned(),
            written: raw.to_owned(),
        })
    }

    /// Правила crates.io; зарезервированные имена — политика, а не грамматика.
    fn parse_cargo(raw: &str) -> Result<Self, NameError> {
        let Some(first) = raw.chars().next() else {
            return Err(NameError::Empty);
        };
        let len = raw.chars().count();
        if len > CARGO_MAX_LEN {
            return Err(NameError::TooLong {
                len,
                max: CARGO_MAX_LEN,
            });
        }
        if !first.is_ascii_alphabetic() {
            return Err(NameError::InvalidStart(first));
        }
        if let Some(bad) = raw
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
        {
            return Err(NameError::InvalidChar(bad));
        }
        // Как canon_crate_name в crates.io: `-` и `_` обозначают одно имя.
        let normalized = raw.to_ascii_lowercase().replace('-', "_");
        Ok(Self {
            normalized,
            written: raw.to_owned(),
        })
    }

    #[must_use]
    pub fn normalized(&self) -> &str {
        &self.normalized
    }

    #[must_use]
    pub fn as_written(&self) -> &str {
        &self.written
    }
}

/// Часть имени npm (scope или имя): не пустая, не с `.`/`_`, только URL-безопасные символы.
fn check_npm_part(part: &str) -> Result<(), NameError> {
    if let Some(first) = part.chars().next().filter(|c| matches!(c, '.' | '_')) {
        return Err(NameError::InvalidStart(first));
    }
    match part
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || "-._~!'()*".contains(*c)))
    {
        Some(bad) => Err(NameError::InvalidChar(bad)),
        None => Ok(()),
    }
}

impl PartialEq for PackageName {
    fn eq(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

impl Eq for PackageName {}

impl Hash for PackageName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.written)
    }
}

#[cfg(test)]
#[path = "tests/name.rs"]
mod tests;
