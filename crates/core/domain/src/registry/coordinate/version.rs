use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use super::Format;
use crate::registry::OrderKey;

/// Ошибка разбора версии.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionError {
    #[error("некорректная версия {raw:?}: {reason}")]
    Invalid { raw: String, reason: String },
    #[error("формат {0:?} пока не поддержан")]
    Unsupported(Format),
}

/// Версия пакета: уникальность задаёт каноническая строка, порядок — ключ `OrderKey`,
/// написанная клиентом форма сохраняется для отдачи.
#[derive(Debug, Clone)]
pub struct Version {
    canonical: String,
    written: String,
    order: OrderKey,
    prerelease: bool,
}

impl Version {
    /// Собирает версию из готовых частей.
    ///
    /// Части уже прошли `Ecosystem::parse_version` или прочитаны из каталога: грамматику
    /// и согласованность `order` с `canonical` здесь никто не проверяет.
    #[must_use]
    pub const fn from_parts(
        canonical: String,
        written: String,
        order: OrderKey,
        prerelease: bool,
    ) -> Self {
        Self {
            canonical,
            written,
            order,
            prerelease,
        }
    }

    /// Есть ли у версии pre-release часть (`1.0.0-rc.1`).
    #[must_use]
    pub const fn is_prerelease(&self) -> bool {
        self.prerelease
    }

    /// Каноническая строка: по ней версия уникальна в БД.
    #[must_use]
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    #[must_use]
    pub fn as_written(&self) -> &str {
        &self.written
    }

    #[must_use]
    pub const fn order_key(&self) -> &OrderKey {
        &self.order
    }

    /// Старшинство только по ключу порядка: версии с разной канонической строкой могут быть
    /// равны (Maven: `1` и `1.0`). Это сравнение для диапазонов advisory, `Ord` для них не годится.
    #[must_use]
    pub fn precedence(&self, other: &Self) -> Ordering {
        self.order.cmp(&other.order)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
    }
}

impl Eq for Version {}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical.hash(state);
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Полный порядок по `(order, canonical)`, согласованный с `Eq`: равными считаются только
/// версии с одной канонической строкой. Не для диапазонов: версии, равные по старшинству,
/// здесь различаются тайбрейком; для диапазонов есть [`Version::precedence`].
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order
            .cmp(&other.order)
            .then_with(|| self.canonical.cmp(&other.canonical))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.written)
    }
}

#[cfg(test)]
#[path = "tests/version.rs"]
mod tests;
