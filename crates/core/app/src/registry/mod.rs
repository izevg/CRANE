//! Use cases bounded context «реестр»: публикация, получение и yank версий.

mod fetch;
mod publish;
mod yank;

use std::fmt;
use std::sync::Arc;

use crane_domain::registry::{
    Asset, Format, FormatMetadata, PackageName, PackageVersion, Repository, Sha256, Version,
    VersionState,
};
use crane_ports::registry::{BlobError, BlobStore, ByteStream, Catalog, CatalogError};
use crane_ports::system::{Clock, IdGen};

/// Координата версии пакета в репозитории.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinate {
    pub format: Format,
    pub repository: String,
    pub package: PackageName,
    pub version: Version,
}

/// Файл версии от клиента. Один файл на запрос: так устроены все протоколы;
/// добавление файлов к версии (`PyPI`, Maven) — отдельный use case позже.
pub struct Upload {
    pub path: String,
    pub body: ByteStream,
    pub declared_sha256: Option<Sha256>,
}

impl fmt::Debug for Upload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Поток не Debug и не должен попадать в журналы.
        f.debug_struct("Upload")
            .field("path", &self.path)
            .field("declared_sha256", &self.declared_sha256)
            .finish_non_exhaustive()
    }
}

/// Запрос на публикацию версии.
#[derive(Debug)]
pub struct Publish {
    pub coordinate: Coordinate,
    pub file: Upload,
    pub metadata: FormatMetadata,
}

/// Скачиваемый файл версии.
pub struct Download {
    pub asset: Asset,
    pub body: ByteStream,
}

impl fmt::Debug for Download {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Download")
            .field("asset", &self.asset)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("репозиторий не найден")]
    RepositoryNotFound,
    /// Публикация разрешена только в hosted-репозиторий.
    #[error("репозиторий не hosted: публикация запрещена")]
    NotHosted,
    #[error("версия не найдена")]
    VersionNotFound,
    #[error("файл версии не найден")]
    AssetNotFound,
    /// В том числе версия, удалённая tombstone-ом: номер релиза занят навсегда.
    #[error("версия уже существует")]
    VersionExists,
    #[error("версия недоступна: {state:?}")]
    Unavailable { state: VersionState },
    #[error("sha256 не совпал: заявлен {declared}, получен {actual}")]
    DigestMismatch { declared: Sha256, actual: Sha256 },
    /// Обрыв тела запроса: ошибка клиента, не хранилища.
    #[error("ошибка чтения тела запроса")]
    Body(#[source] std::io::Error),
    /// Сбой хранилища или нарушенный инвариант: для клиента это 500.
    #[error("ошибка бэкенда")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

// Единственное место, где ошибки портов превращаются в ошибки use case.
impl From<CatalogError> for RegistryError {
    fn from(error: CatalogError) -> Self {
        match error {
            CatalogError::VersionExists => Self::VersionExists,
            // `find_*` возвращают Option, а записи физически не удаляются, поэтому
            // NotFound и StateConflict здесь означают нарушенный инвариант, то есть 500.
            CatalogError::NotFound
            | CatalogError::StateConflict { .. }
            | CatalogError::Backend(_) => Self::Backend(Box::new(error)),
        }
    }
}

impl From<BlobError> for RegistryError {
    fn from(error: BlobError) -> Self {
        match error {
            BlobError::Body(source) => Self::Body(source),
            // Нет blob при существующей записи каталога — нарушенный инвариант (500),
            // а не AssetNotFound: клиент просил файл, который каталог обещает.
            BlobError::NotFound(_) | BlobError::Backend(_) => Self::Backend(Box::new(error)),
        }
    }
}

/// Use cases реестра над каталогом и хранилищем blob.
pub struct Registry<C, B> {
    catalog: C,
    blobs: B,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGen>,
}

impl<C, B> fmt::Debug for Registry<C, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Порты и `dyn Clock` не Debug.
        f.debug_struct("Registry").finish_non_exhaustive()
    }
}

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    #[must_use]
    pub fn new(catalog: C, blobs: B, clock: Arc<dyn Clock>, ids: Arc<dyn IdGen>) -> Self {
        Self {
            catalog,
            blobs,
            clock,
            ids,
        }
    }

    async fn repository(&self, format: Format, name: &str) -> Result<Repository, RegistryError> {
        self.catalog
            .find_repository(format, name)
            .await?
            .ok_or(RegistryError::RepositoryNotFound)
    }

    async fn find(&self, coordinate: &Coordinate) -> Result<PackageVersion, RegistryError> {
        let repository = self
            .repository(coordinate.format, &coordinate.repository)
            .await?;
        self.catalog
            .find_version(repository.id, &coordinate.package, &coordinate.version)
            .await?
            .ok_or(RegistryError::VersionNotFound)
    }
}

/// Видимость версии для клиентов: видна только `Released`.
/// Yanked ортогонален состоянию и версию не скрывает.
/// Единственное правило видимости: и `fetch`, и `versions` идут через него.
fn served(version: &PackageVersion) -> Result<(), RegistryError> {
    match version.state {
        VersionState::Released => Ok(()),
        state @ (VersionState::Quarantined | VersionState::Blocked | VersionState::Deleted) => {
            Err(RegistryError::Unavailable { state })
        }
    }
}

/// Use cases дают Send-future: entry исполняет их на многопоточном tokio.
/// Проверка при компиляции и для любых адаптеров, а не только memory.
#[cfg(test)]
#[expect(
    dead_code,
    reason = "проверка типов: функцию не вызывают, её компилируют"
)]
fn use_case_futures_are_send<C: Catalog, B: BlobStore>(
    registry: &Registry<C, B>,
    publish: Publish,
    coordinate: &Coordinate,
    package: &PackageName,
) {
    fn send<T: Send>(_: T) {}
    send(registry.publish(publish));
    send(registry.fetch(coordinate, ""));
    send(registry.versions(Format::Cargo, "", package));
    send(registry.set_yanked(coordinate, true));
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use std::io;

    use super::*;

    fn sha(byte: u8) -> Sha256 {
        Sha256::from([byte; 32])
    }

    fn is_backend(error: &RegistryError) -> bool {
        matches!(error, RegistryError::Backend(_))
    }

    #[test]
    fn catalog_version_exists_maps_to_version_exists() {
        // Arrange
        let error = CatalogError::VersionExists;

        // Act
        let mapped = RegistryError::from(error);

        // Assert
        assert!(
            matches!(mapped, RegistryError::VersionExists),
            "tombstone и дубликат должны давать 409, а не 500: {mapped:?}"
        );
    }

    #[test]
    fn catalog_broken_invariants_map_to_backend() {
        // Arrange
        let errors = [
            CatalogError::NotFound,
            CatalogError::StateConflict {
                actual: VersionState::Deleted,
            },
            CatalogError::Backend("сбой".into()),
        ];

        // Act
        let mapped = errors.map(RegistryError::from);

        // Assert
        assert!(
            mapped.iter().all(is_backend),
            "прочие ошибки каталога — 500: {mapped:?}"
        );
    }

    #[test]
    fn blob_body_error_maps_to_body() {
        // Arrange
        let error = BlobError::Body(io::Error::other("обрыв"));

        // Act
        let mapped = RegistryError::from(error);

        // Assert
        assert!(
            matches!(mapped, RegistryError::Body(_)),
            "обрыв тела — ошибка клиента: {mapped:?}"
        );
    }

    #[test]
    fn other_blob_errors_map_to_backend() {
        // Arrange
        let errors = [
            BlobError::NotFound(sha(1)),
            BlobError::Backend("сбой".into()),
        ];

        // Act
        let mapped = errors.map(RegistryError::from);

        // Assert
        assert!(
            mapped.iter().all(is_backend),
            "отсутствующий blob при записи каталога — 500, не AssetNotFound: {mapped:?}"
        );
    }
}
