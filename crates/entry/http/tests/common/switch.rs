//! Каталог теста с выключателем: пока «лежит», поиски имени отвечают `Unreachable`, как при
//! недоступной БД; остальное делегируется памяти. Нужен, чтобы проверить отказ защиты имён в
//! закрытую сторону через настоящие маршруты.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crane_domain::registry::{
    Format, Namespace, PackageName, PackageVersion, Repository, RepositoryId, Version, VersionId,
    VersionState,
};
use crane_memory::MemoryCatalog;
use crane_ports::registry::{Catalog, CatalogError, NewVersion};

#[derive(Debug)]
pub(crate) struct SwitchCatalog {
    inner: MemoryCatalog,
    down: Arc<AtomicBool>,
}

impl SwitchCatalog {
    pub(crate) fn new(inner: MemoryCatalog, down: Arc<AtomicBool>) -> Self {
        Self { inner, down }
    }

    fn is_down(&self) -> bool {
        self.down.load(Ordering::SeqCst)
    }
}

fn unreachable() -> CatalogError {
    CatalogError::Unreachable("БД недоступна".into())
}

impl Catalog for SwitchCatalog {
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
        self.inner.find_version(repository, package, version)
    }

    fn set_yanked(
        &self,
        id: VersionId,
        yanked: bool,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.inner.set_yanked(id, yanked)
    }

    fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.inner.transition(id, from, to)
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

    async fn namespace_for(
        &self,
        format: Format,
        name: &PackageName,
    ) -> Result<Option<Namespace>, CatalogError> {
        if self.is_down() {
            return Err(unreachable());
        }
        self.inner.namespace_for(format, name).await
    }

    async fn hosted_exists(
        &self,
        format: Format,
        name: &PackageName,
    ) -> Result<bool, CatalogError> {
        if self.is_down() {
            return Err(unreachable());
        }
        self.inner.hosted_exists(format, name).await
    }

    fn ensure_namespace(
        &self,
        namespace: Namespace,
    ) -> impl Future<Output = Result<Namespace, CatalogError>> + Send {
        self.inner.ensure_namespace(namespace)
    }
}
