//! Адрес версии пакета в реестре.

use super::{Format, PackageName, Version};

/// Адрес пакета (без версии) в репозитории: цель действий над пакетом целиком.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    pub format: Format,
    pub repository: String,
    pub package: PackageName,
}

/// Координата версии пакета в репозитории.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinate {
    pub format: Format,
    pub repository: String,
    pub package: PackageName,
    pub version: Version,
}
