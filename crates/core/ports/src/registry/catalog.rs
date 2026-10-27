use std::future::Future;
use std::time::SystemTime;

use crane_domain::{
    Asset, Format, FormatMetadata, PackageName, PackageVersion, Repository, RepositoryId, Version,
    VersionId, VersionState,
};

/// Версия к публикации. `yanked` при публикации всегда `false`, поэтому поля нет.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewVersion {
    pub id: VersionId,
    pub repository: RepositoryId,
    pub package: PackageName,
    pub version: Version,
    pub state: VersionState,
    pub published_at: SystemTime,
    pub assets: Vec<Asset>,
    pub metadata: FormatMetadata,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// Нет репозитория или версии.
    #[error("запись не найдена")]
    NotFound,
    /// Версия уже занята, в том числе tombstone (`Deleted`).
    #[error("версия уже существует")]
    VersionExists,
    /// Compare-and-set не сошёлся: фактическое состояние отличается от ожидаемого.
    #[error("конфликт состояния: фактическое {actual:?}")]
    StateConflict { actual: VersionState },
    #[error("ошибка каталога")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Каталог реестра: репозитории, пакеты, версии и их жизненный цикл.
pub trait Catalog: Send + Sync {
    /// Идемпотентно: если репозиторий с тем же `(format, name)` есть, возвращает сохранённый,
    /// а переданный id игнорируется.
    fn ensure_repository(
        &self,
        repository: Repository,
    ) -> impl Future<Output = Result<Repository, CatalogError>> + Send;

    fn find_repository(
        &self,
        format: Format,
        name: &str,
    ) -> impl Future<Output = Result<Option<Repository>, CatalogError>> + Send;

    /// Атомарно. Пакет создаётся при первой версии и хранит первое написание имени.
    /// Версия уникальна по `(repository, name.normalized(), version.canonical())` навсегда,
    /// включая `Deleted`. Неизвестный репозиторий даёт `NotFound`.
    /// Возвращённая запись несёт сохранённое написание пакета и `yanked = false`.
    fn publish_version(
        &self,
        version: NewVersion,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send;

    /// Все версии пакета, включая `Deleted`, в порядке публикации.
    /// Нет пакета: пустой вектор.
    fn versions(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<PackageVersion>, CatalogError>> + Send;

    fn find_version(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        version: &Version,
    ) -> impl Future<Output = Result<Option<PackageVersion>, CatalogError>> + Send;

    /// Идемпотентно. Неизвестный id даёт `NotFound`.
    fn set_yanked(
        &self,
        id: VersionId,
        yanked: bool,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send;

    /// Compare-and-set: меняет состояние, только если текущее равно `from`,
    /// иначе `StateConflict { actual }`. Допустимость ребра проверяет вызывающий
    /// (`VersionState::ensure`), адаптер — нет. Неизвестный id даёт `NotFound`.
    fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send;
}
