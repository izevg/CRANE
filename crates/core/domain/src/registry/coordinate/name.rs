use std::fmt;
use std::hash::{Hash, Hasher};

use super::Format;

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
    /// Собирает имя из готовых частей.
    ///
    /// Части уже прошли `Ecosystem::parse_name` или прочитаны из каталога: грамматику
    /// и нормализацию здесь никто не проверяет.
    #[must_use]
    pub const fn from_parts(normalized: String, written: String) -> Self {
        Self {
            normalized,
            written,
        }
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
