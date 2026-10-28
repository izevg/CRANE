use std::num::NonZeroU64;
use std::time::SystemTime;

use super::Secret;

/// Версия строки для оптимистичной блокировки (CAS). Начинается с 1 и только растёт.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(NonZeroU64);

impl Version {
    pub const FIRST: Self = Self(NonZeroU64::MIN);

    /// `None` для нуля: версии 0 нет, строка с ней не создаётся.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(version) => Some(Self(version)),
            None => None,
        }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Следующая версия; на переполнении `None`, а не паника и не оборот к нулю.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Значение настройки в хранилище. Без `Debug`: ручной `Debug` с редакцией исключён
/// из мутационной проверки (`.cargo/mutants.toml`) и скрыл бы мутанта редакции.
#[allow(
    missing_debug_implementations,
    reason = "Debug значения настройки мог бы раскрыть секрет"
)]
pub enum StoredValue {
    Plain(String),
    Secret(Secret),
}

/// Строка хранилища настроек. Без `Debug` по той же причине, что и `StoredValue`.
#[allow(
    missing_debug_implementations,
    reason = "Debug строки мог бы раскрыть секрет"
)]
pub struct Stored {
    pub value: StoredValue,
    pub version: Version,
    pub updated_at: SystemTime,
    pub updated_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_increments_version_by_one() {
        // Arrange
        let cases = [Version::FIRST, Version::new(41).unwrap_or(Version::FIRST)];

        // Act
        let actual = cases.map(|version| version.next().map(Version::get));

        // Assert
        assert_eq!(actual, [Some(2), Some(42)]);
    }

    #[test]
    fn next_of_max_version_overflows_to_none() {
        // Arrange
        let max = Version::new(u64::MAX);

        // Act
        let overflow = max.map(Version::next);

        // Assert
        assert_eq!(overflow, Some(None));
    }
}
