//! Use cases bounded context «реестр»: публикация, получение, yank, удаление, устаревание и теги версий.

mod flags;
mod read;
mod write;

use std::fmt;
use std::sync::Arc;

use crane_domain::access::Caller;
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
pub use crane_domain::registry::Coordinate;
use crane_domain::registry::{
    Asset, Checksum, Format, FormatMetadata, PackageVersion, Repository, RepositoryKind, Sha256,
    VersionId, VersionState,
};
use crane_ports::audit::AuditSink;
use crane_ports::registry::{BlobError, BlobStore, ByteStream, Catalog, CatalogError};
use crane_ports::system::{Clock, IdGen};

/// Файл версии от клиента. Один файл на запрос: так устроены все протоколы;
/// добавление файлов к версии (`PyPI`, Maven) — отдельный use case позже.
pub struct Upload {
    pub path: String,
    pub body: ByteStream,
    /// Контрольные суммы, заявленные клиентом: проверяется каждая, пустой список — заявлений нет.
    pub declared: Vec<Checksum>,
}

impl fmt::Debug for Upload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Поток не Debug и не должен попадать в журналы.
        f.debug_struct("Upload")
            .field("path", &self.path)
            .field("declared", &self.declared)
            .finish_non_exhaustive()
    }
}

/// Запрос на публикацию версии.
#[derive(Debug)]
pub struct Publish {
    pub coordinate: Coordinate,
    pub file: Upload,
    pub metadata: FormatMetadata,
    /// Dist-теги, которые публикация ставит на версию (переносит с прежних).
    pub tags: Vec<String>,
}

/// Пакет для чтения: видимые клиентам версии и теги. Фильтр тегов по видимым версиям — дело формата.
#[derive(Debug)]
pub struct PackageView {
    pub versions: Vec<PackageVersion>,
    /// Отсортированы по имени тега; могут указывать на невидимые версии.
    pub tags: Vec<(String, VersionId)>,
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
    // Checksum с sha512 велик: без Box вариант раздувает каждый `Result<_, RegistryError>`.
    #[error("контрольная сумма не совпала: заявлена {declared}, получена {actual}")]
    DigestMismatch {
        declared: Box<Checksum>,
        actual: Box<Checksum>,
    },
    /// Окно, в течение которого версию можно удалить, закрыто: остаётся блокировка.
    #[error("окно удаления закрыто: версия опубликована слишком давно")]
    DeleteWindowClosed,
    #[error("тег не найден")]
    TagNotFound,
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
    audit: Arc<dyn AuditSink>,
}

impl<C, B> fmt::Debug for Registry<C, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Порты и `dyn`-зависимости не Debug.
        f.debug_struct("Registry").finish_non_exhaustive()
    }
}

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    #[must_use]
    pub fn new(
        catalog: C,
        blobs: B,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGen>,
        audit: Arc<dyn AuditSink>,
    ) -> Self {
        Self {
            catalog,
            blobs,
            clock,
            ids,
            audit,
        }
    }

    /// Репозиторий по формату и имени. Чтение, аудита нет.
    pub async fn repository(
        &self,
        // Чтение пока не авторизуется; в v0.4 Cedar решает и для чтений.
        _caller: &Caller,
        format: Format,
        name: &str,
    ) -> Result<Repository, RegistryError> {
        self.find_repository(format, name).await
    }

    /// Готовность хранилищ для пробы оркестратора: вызывающего и аудита нет.
    ///
    /// Вопрос задаётся через существующие контракты портов, портам не нужен отдельный `ping`.
    /// Каталогу: `find_repository` для имени, которого не бывает. Любой `Ok` означает, что
    /// бэкенд ответил. Blob-хранилищу: `open` нулевого дайджеста. Такого blob в CAS нет, поэтому
    /// `NotFound` тоже означает «бэкенд ответил»; готовность ломает только другая ошибка.
    ///
    /// # Errors
    ///
    /// `Backend`, если каталог или blob-хранилище не ответили.
    pub async fn ready(&self) -> Result<(), RegistryError> {
        self.catalog.find_repository(Format::Cargo, "").await?;
        match self.blobs.open(&Sha256::from([0; 32])).await {
            Ok(_) | Err(BlobError::NotFound(_)) => Ok(()),
            Err(error) => Err(RegistryError::Backend(Box::new(error))),
        }
    }

    /// Единственная точка эмиссии аудита: ровно одно событие на мутирующий use case.
    fn audited<T>(&self, caller: &Caller, action: AuditAction, result: &Result<T, RegistryError>) {
        let outcome = match result {
            Ok(_) => Outcome::Success,
            // Только Display: цепочка `source` у Backend может содержать адреса хранилища.
            Err(error) => Outcome::Failure {
                reason: error.to_string(),
            },
        };
        self.audit.record(AuditEvent {
            actor: Some(caller.principal.clone()),
            request: caller.request.clone(),
            action,
            outcome,
        });
    }

    async fn find_repository(
        &self,
        format: Format,
        name: &str,
    ) -> Result<Repository, RegistryError> {
        self.catalog
            .find_repository(format, name)
            .await?
            .ok_or(RegistryError::RepositoryNotFound)
    }

    /// Версия hosted-репозитория: мутации, меняющие содержимое, прокси-репозиториям запрещены.
    async fn find_hosted(&self, coordinate: &Coordinate) -> Result<PackageVersion, RegistryError> {
        let repository = self
            .find_repository(coordinate.format, &coordinate.repository)
            .await?;
        if repository.kind != RepositoryKind::Hosted {
            return Err(RegistryError::NotHosted);
        }
        self.catalog
            .find_version(repository.id, &coordinate.package, &coordinate.version)
            .await?
            .ok_or(RegistryError::VersionNotFound)
    }

    async fn find(&self, coordinate: &Coordinate) -> Result<PackageVersion, RegistryError> {
        let repository = self
            .find_repository(coordinate.format, &coordinate.repository)
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
    caller: &Caller,
    publish: Publish,
    coordinate: &Coordinate,
    package_ref: &crane_domain::registry::PackageRef,
    policy: crane_domain::registry::DeletePolicy,
) {
    fn send<T: Send>(_: T) {}
    let package = &package_ref.package;
    send(registry.publish(caller, publish));
    send(registry.fetch(caller, coordinate, ""));
    send(registry.versions(caller, Format::Cargo, "", package));
    send(registry.set_yanked(caller, coordinate, true));
    send(registry.delete(caller, coordinate, &policy));
    send(registry.delete_package(caller, Format::Cargo, "", package, &policy));
    send(registry.set_deprecated(caller, coordinate, None));
    send(registry.set_tag(caller, coordinate, ""));
    send(registry.remove_tag(caller, package_ref, ""));
    send(registry.package(caller, Format::Cargo, "", package));
    send(registry.repository(caller, Format::Cargo, ""));
    send(registry.ready());
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
