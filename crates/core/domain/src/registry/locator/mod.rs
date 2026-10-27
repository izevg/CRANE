//! Адрес версии пакета в реестре.

use super::{Format, PackageName, Version};

/// Координата версии пакета в репозитории.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinate {
    pub format: Format,
    pub repository: String,
    pub package: PackageName,
    pub version: Version,
}
