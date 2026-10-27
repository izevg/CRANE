//! Proxy-репозитории: политики кэша upstream и защита исходящих запросов (SSRF).
//!
//! Модуль чистый: сроки считаются над фактами записи (когда получена, сколько раз не менялась),
//! а тело и `ETag` живут в портах. Пресетов TTL для форматов здесь нет: это константы крейтов форматов.

mod egress;
mod path;
mod policy;

pub use egress::{EgressPolicy, EgressRefusal, Scheme, UpstreamBase};
pub use path::{PathError, ProxyKey, UpstreamPath};
pub use policy::{Freshness, MetadataPolicy, NegativePolicy, age};

use crate::registry::RepositoryId;

/// Источник proxy-репозитория: кому принадлежит кэш и куда ходить.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    pub repository: RepositoryId,
    pub base: UpstreamBase,
}

impl Origin {
    /// Ключ кэша выводится из источника, а не передаётся отдельно: нельзя положить чужой путь под чужой репозиторий.
    #[must_use]
    pub fn key(&self, path: &UpstreamPath) -> ProxyKey {
        ProxyKey {
            repository: self.repository,
            path: path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn key_binds_path_to_origin_repository() {
        // Arrange
        let repository = RepositoryId::new(Uuid::from_u128(7));
        let policy = EgressPolicy::new(["index.crates.io"], false);
        let base = policy.admit("https", "index.crates.io", None, "").unwrap();
        let origin = Origin { repository, base };
        let path = UpstreamPath::parse("se/rd/serde").unwrap();

        // Act
        let key = origin.key(&path);

        // Assert
        assert_eq!(key, ProxyKey { repository, path });
    }
}
