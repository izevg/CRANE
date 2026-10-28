//! Декораторы адаптеров для сценариев отказов: слепая предпроверка, недоступная БД, сбои blob-хранилища.

use std::future::{Future, ready};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crane_domain::registry::{
    Asset, Format, Namespace, NamespacePattern, PackageName, PackageVersion, Repository,
    RepositoryId, Sha256, Version, VersionId, VersionState,
};
use crane_memory::{MemoryBlobStore, MemoryCatalog};
use crane_ports::registry::{
    BlobError, BlobStore, ByteStream, Catalog, CatalogError, Digests, NewVersion, Staged,
};

/// Каталог с «слепой» предпроверкой: первые `blind_calls` вызовов `find_version` дают `None`,
/// остальное делегируется. Моделирует гонку: между предпроверкой и `publish_version`
/// версию занял другой запрос.
#[derive(Debug)]
pub(crate) struct BlindCatalog {
    inner: MemoryCatalog,
    blind_calls: AtomicUsize,
    /// Пакет тоже «не виден»: оба конкурента первой загрузки считают имя свободным.
    hides_package: bool,
}

impl BlindCatalog {
    /// Предпроверка слепа всегда.
    pub(crate) fn always(inner: MemoryCatalog) -> Self {
        Self {
            inner,
            blind_calls: AtomicUsize::new(usize::MAX),
            hides_package: false,
        }
    }

    /// Гонка двух первых загрузок: первая предпроверка версии слепа и пакета «ещё нет»,
    /// повторное чтение уже видит версию-победителя.
    pub(crate) fn once(inner: MemoryCatalog) -> Self {
        Self {
            inner,
            blind_calls: AtomicUsize::new(1),
            hides_package: true,
        }
    }

    fn is_blind(&self) -> bool {
        self.blind_calls
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            })
            .is_ok()
    }
}

impl Catalog for BlindCatalog {
    fn ensure_repository(
        &self,
        repository: Repository,
    ) -> impl Future<Output = Result<Repository, CatalogError>> + Send {
        self.inner.ensure_repository(repository)
    }

    fn find_repository(
        &self,
        format: Format,
        name: &str,
    ) -> impl Future<Output = Result<Option<Repository>, CatalogError>> + Send {
        self.inner.find_repository(format, name)
    }

    fn publish_version(
        &self,
        version: NewVersion,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        self.inner.publish_version(version)
    }

    fn add_asset(
        &self,
        version: VersionId,
        asset: Asset,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        self.inner.add_asset(version, asset)
    }

    fn versions(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<PackageVersion>, CatalogError>> + Send {
        self.inner.versions(repository, package)
    }

    fn find_version(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        version: &Version,
    ) -> impl Future<Output = Result<Option<PackageVersion>, CatalogError>> + Send {
        let blind = self.is_blind();
        let found = self.inner.find_version(repository, package, version);
        async move { if blind { Ok(None) } else { found.await } }
    }

    fn set_yanked(
        &self,
        id: VersionId,
        yanked: bool,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.inner.set_yanked(id, yanked)
    }

    fn set_deprecated(
        &self,
        id: VersionId,
        message: Option<String>,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.inner.set_deprecated(id, message)
    }

    fn set_tag(
        &self,
        version: VersionId,
        tag: &str,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.inner.set_tag(version, tag)
    }

    fn remove_tag(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        tag: &str,
    ) -> impl Future<Output = Result<bool, CatalogError>> + Send {
        self.inner.remove_tag(repository, package, tag)
    }

    fn tags(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<(String, VersionId)>, CatalogError>> + Send {
        self.inner.tags(repository, package)
    }

    fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.inner.transition(id, from, to)
    }

    fn namespace_for(
        &self,
        format: Format,
        candidates: &[NamespacePattern],
    ) -> impl Future<Output = Result<Option<Namespace>, CatalogError>> + Send {
        self.inner.namespace_for(format, candidates)
    }

    fn hosted_exists(
        &self,
        format: Format,
        name: &PackageName,
    ) -> impl Future<Output = Result<bool, CatalogError>> + Send {
        let hidden = self.hides_package;
        let exists = self.inner.hosted_exists(format, name);
        async move { if hidden { Ok(false) } else { exists.await } }
    }

    fn ensure_namespace(
        &self,
        namespace: Namespace,
    ) -> impl Future<Output = Result<Namespace, CatalogError>> + Send {
        self.inner.ensure_namespace(namespace)
    }
}

/// Каталог, который не отвечает на `find_repository`; остальное делегируется.
/// Моделирует недоступную БД для пробы готовности.
#[derive(Debug)]
pub(crate) struct DownCatalog(pub MemoryCatalog);

impl Catalog for DownCatalog {
    fn ensure_repository(
        &self,
        repository: Repository,
    ) -> impl Future<Output = Result<Repository, CatalogError>> + Send {
        self.0.ensure_repository(repository)
    }

    fn find_repository(
        &self,
        _format: Format,
        _name: &str,
    ) -> impl Future<Output = Result<Option<Repository>, CatalogError>> + Send {
        ready(Err(CatalogError::Unreachable("БД недоступна".into())))
    }

    fn publish_version(
        &self,
        version: NewVersion,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        self.0.publish_version(version)
    }

    fn add_asset(
        &self,
        version: VersionId,
        asset: Asset,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        self.0.add_asset(version, asset)
    }

    fn versions(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<PackageVersion>, CatalogError>> + Send {
        self.0.versions(repository, package)
    }

    fn find_version(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        version: &Version,
    ) -> impl Future<Output = Result<Option<PackageVersion>, CatalogError>> + Send {
        self.0.find_version(repository, package, version)
    }

    fn set_yanked(
        &self,
        id: VersionId,
        yanked: bool,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.0.set_yanked(id, yanked)
    }

    fn set_deprecated(
        &self,
        id: VersionId,
        message: Option<String>,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.0.set_deprecated(id, message)
    }

    fn set_tag(
        &self,
        version: VersionId,
        tag: &str,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.0.set_tag(version, tag)
    }

    fn remove_tag(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        tag: &str,
    ) -> impl Future<Output = Result<bool, CatalogError>> + Send {
        self.0.remove_tag(repository, package, tag)
    }

    fn tags(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<(String, VersionId)>, CatalogError>> + Send {
        self.0.tags(repository, package)
    }

    fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.0.transition(id, from, to)
    }

    fn namespace_for(
        &self,
        format: Format,
        candidates: &[NamespacePattern],
    ) -> impl Future<Output = Result<Option<Namespace>, CatalogError>> + Send {
        self.0.namespace_for(format, candidates)
    }

    fn hosted_exists(
        &self,
        format: Format,
        name: &PackageName,
    ) -> impl Future<Output = Result<bool, CatalogError>> + Send {
        self.0.hosted_exists(format, name)
    }

    fn ensure_namespace(
        &self,
        namespace: Namespace,
    ) -> impl Future<Output = Result<Namespace, CatalogError>> + Send {
        self.0.ensure_namespace(namespace)
    }
}

/// Blob-хранилище, у которого `open` отвечает заданной ошибкой; остальное делегируется.
#[derive(Debug)]
pub(crate) struct OpenFailsBlobs {
    inner: MemoryBlobStore,
    error: fn() -> BlobError,
}

impl OpenFailsBlobs {
    pub(crate) fn new(inner: MemoryBlobStore, error: fn() -> BlobError) -> Self {
        Self { inner, error }
    }
}

impl BlobStore for OpenFailsBlobs {
    fn stage(&self, body: ByteStream) -> impl Future<Output = Result<Staged, BlobError>> + Send {
        self.inner.stage(body)
    }

    fn commit(&self, staged: Staged) -> impl Future<Output = Result<Digests, BlobError>> + Send {
        self.inner.commit(staged)
    }

    fn open(&self, _sha256: &Sha256) -> impl Future<Output = Result<ByteStream, BlobError>> + Send {
        ready(Err((self.error)()))
    }
}

/// Журнал попыток commit, общий с тестом.
pub(crate) type CommitLog = Arc<Mutex<Vec<Sha256>>>;

/// Blob-хранилище, записывающее попытки commit; при `fail_commit` commit завершается ошибкой.
#[derive(Debug)]
pub(crate) struct RecordingBlobs {
    inner: MemoryBlobStore,
    commits: CommitLog,
    fail_commit: bool,
}

impl RecordingBlobs {
    pub(crate) fn recording(inner: MemoryBlobStore, commits: CommitLog) -> Self {
        Self {
            inner,
            commits,
            fail_commit: false,
        }
    }

    pub(crate) fn failing_commit(inner: MemoryBlobStore, commits: CommitLog) -> Self {
        Self {
            inner,
            commits,
            fail_commit: true,
        }
    }
}

pub(crate) fn commit_log() -> CommitLog {
    Arc::default()
}

pub(crate) fn committed(log: &CommitLog) -> Vec<Sha256> {
    log.lock().unwrap().clone()
}

impl BlobStore for RecordingBlobs {
    fn stage(&self, body: ByteStream) -> impl Future<Output = Result<Staged, BlobError>> + Send {
        self.inner.stage(body)
    }

    fn commit(&self, staged: Staged) -> impl Future<Output = Result<Digests, BlobError>> + Send {
        self.commits.lock().unwrap().push(staged.digests().sha256);
        let fail = self.fail_commit;
        let inner = &self.inner;
        async move {
            if fail {
                Err(BlobError::Backend("хранилище отказало".into()))
            } else {
                inner.commit(staged).await
            }
        }
    }

    fn open(&self, sha256: &Sha256) -> impl Future<Output = Result<ByteStream, BlobError>> + Send {
        self.inner.open(sha256)
    }
}
