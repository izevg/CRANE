use std::time::SystemTime;

use super::{RepositoryId, VersionId};
use crate::registry::{PackageName, Sha256, Version, VersionState};

/// Файл версии в CAS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub path: String,
    pub sha256: Sha256,
    pub size: u64,
}

/// Непрозрачный JSON формата: ядро его не читает и не проверяет.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatMetadata(String);

impl FormatMetadata {
    #[must_use]
    pub const fn new(json: String) -> Self {
        Self(json)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Версия пакета в репозитории.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVersion {
    pub id: VersionId,
    pub repository: RepositoryId,
    pub package: PackageName,
    pub version: Version,
    pub state: VersionState,
    /// Ортогонален состоянию (PEP 592, Cargo yank).
    pub yanked: bool,
    pub published_at: SystemTime,
    pub assets: Vec<Asset>,
    pub metadata: FormatMetadata,
}
