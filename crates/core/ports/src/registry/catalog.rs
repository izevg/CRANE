use std::future::Future;
use std::time::SystemTime;

use crane_domain::registry::{
    Asset, Format, FormatMetadata, Namespace, PackageName, PackageVersion, Repository,
    RepositoryId, Version, VersionId, VersionState,
};

/// Версия к публикации. `yanked` при публикации всегда `false`, а `deprecated` — `None`, поэтому полей нет.
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
    /// Dist-теги, которые публикация ставит на эту версию в той же транзакции (переносит с других версий).
    pub tags: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// Нет репозитория или версии.
    #[error("запись не найдена")]
    NotFound,
    /// Версия уже занята, в том числе tombstone (`Deleted`).
    #[error("версия уже существует")]
    VersionExists,
    /// У версии уже есть файл с этим путём.
    #[error("файл версии уже существует")]
    AssetExists,
    /// Compare-and-set не сошёлся: фактическое состояние отличается от ожидаемого.
    #[error("конфликт состояния: фактическое {actual:?}")]
    StateConflict { actual: VersionState },
    /// Каталог не отвечает (соединение, пул, остановка сервера): повтор позже имеет смысл.
    /// Не путать с `Backend`: тот означает баг или повреждение, повтор не поможет.
    #[error("хранилище не отвечает")]
    Unreachable(#[source] Box<dyn std::error::Error + Send + Sync>),
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
    /// Теги из `NewVersion.tags` ставятся в той же транзакции и переносятся с прежних версий;
    /// при `VersionExists` и `NotFound` не меняется ничего.
    /// Возвращённая запись несёт сохранённое написание пакета, `yanked = false` и `deprecated = None`.
    fn publish_version(
        &self,
        version: NewVersion,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send;

    /// Прикрепляет файл к существующей версии. Путь уже занят у этой версии: `AssetExists`,
    /// не меняется ничего; тот же путь у другой версии не мешает. Неизвестный id: `NotFound`.
    /// Состояние версии не проверяет: это правило вызывающего (как у `transition`).
    /// Blob регистрируется так же, как в `publish_version` (идемпотентно по содержимому),
    /// всё в одной транзакции. Возвращает сохранённую версию со всеми файлами в том же порядке,
    /// что у `find_version`.
    fn add_asset(
        &self,
        version: VersionId,
        asset: Asset,
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

    /// Ставит (`Some`) или снимает (`None`) сообщение об устаревании. Идемпотентно.
    /// Неизвестный id даёт `NotFound`.
    fn set_deprecated(
        &self,
        id: VersionId,
        message: Option<String>,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send;

    /// Ставит dist-тег на версию; пакет определяется по версии, поэтому тег чужого пакета
    /// выразить нельзя. Если тег уже стоит на другой версии, переносит его. Идемпотентно.
    /// Неизвестный id даёт `NotFound`.
    fn set_tag(
        &self,
        version: VersionId,
        tag: &str,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send;

    /// Снимает тег. `false`, если такого тега у пакета не было.
    fn remove_tag(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        tag: &str,
    ) -> impl Future<Output = Result<bool, CatalogError>> + Send;

    /// Теги пакета, отсортированные по имени тега побайтно. Нет пакета: пустой вектор.
    /// Теги могут указывать на версии в любом состоянии, включая `Deleted`: фильтрует формат.
    fn tags(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<(String, VersionId)>, CatalogError>> + Send;

    /// Compare-and-set: меняет состояние, только если текущее равно `from`,
    /// иначе `StateConflict { actual }`. Допустимость ребра проверяет вызывающий
    /// (`VersionState::ensure`), адаптер — нет. Неизвестный id даёт `NotFound`.
    fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send;

    /// Самый специфичный namespace формата, покрывающий имя: перебор в порядке
    /// `NamespacePattern::covering` (точное имя, затем от длинного префикса к короткому),
    /// возвращается первое сохранённое совпадение. Нет покрытия: `None`.
    fn namespace_for(
        &self,
        format: Format,
        name: &PackageName,
    ) -> impl Future<Output = Result<Option<Namespace>, CatalogError>> + Send;

    /// Есть ли пакет с этим нормализованным именем хотя бы в одном hosted-репозитории формата
    /// (версий может быть сколько угодно, в том числе только `Deleted`).
    ///
    /// Несущий инвариант: строки пакетов не удаляются никогда (исчезают только версии,
    /// остаются tombstone), поэтому строка пакета и есть резервирование имени, а защита
    /// от dependency confusion опирается именно на это.
    fn hosted_exists(
        &self,
        format: Format,
        name: &PackageName,
    ) -> impl Future<Output = Result<bool, CatalogError>> + Send;

    /// Идемпотентно по `(format, pattern)`: если namespace есть, возвращает сохранённый
    /// (владелец остаётся прежним), а переданный владелец игнорируется.
    fn ensure_namespace(
        &self,
        namespace: Namespace,
    ) -> impl Future<Output = Result<Namespace, CatalogError>> + Send;
}
