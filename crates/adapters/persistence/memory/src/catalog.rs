use std::future::ready;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crane_domain::registry::{
    Format, PackageName, PackageVersion, Repository, RepositoryId, Version, VersionId, VersionState,
};
use crane_ports::registry::{Catalog, CatalogError, NewVersion};

/// Каталог в памяти.
#[derive(Debug, Default)]
pub struct MemoryCatalog {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    repositories: Vec<Repository>,
    /// В порядке публикации, как того требует контракт `versions`.
    versions: Vec<PackageVersion>,
}

impl MemoryCatalog {
    fn lock(&self) -> MutexGuard<'_, State> {
        // Отравление замка не портит данные: каждая операция атомарна внутри замка.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn update(
        &self,
        id: VersionId,
        change: impl FnOnce(&mut PackageVersion) -> Result<(), CatalogError>,
    ) -> Result<(), CatalogError> {
        let mut state = self.lock();
        let record = state
            .versions
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(CatalogError::NotFound)?;
        change(record)
    }
}

fn same_package(v: &PackageVersion, repository: RepositoryId, package: &PackageName) -> bool {
    v.repository == repository && v.package == *package
}

// Внутри нет `.await`, а `async fn` без него не проходит `clippy::unused_async`,
// поэтому методы возвращают готовые футуры.
impl Catalog for MemoryCatalog {
    fn ensure_repository(
        &self,
        repository: Repository,
    ) -> impl Future<Output = Result<Repository, CatalogError>> + Send {
        let mut state = self.lock();
        let existing = state
            .repositories
            .iter()
            .find(|r| r.format == repository.format && r.name == repository.name)
            .cloned();
        let stored = existing.unwrap_or_else(|| {
            state.repositories.push(repository.clone());
            repository
        });
        ready(Ok(stored))
    }

    fn find_repository(
        &self,
        format: Format,
        name: &str,
    ) -> impl Future<Output = Result<Option<Repository>, CatalogError>> + Send {
        let found = self
            .lock()
            .repositories
            .iter()
            .find(|r| r.format == format && r.name == name)
            .cloned();
        ready(Ok(found))
    }

    fn publish_version(
        &self,
        version: NewVersion,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        let mut state = self.lock();
        if !state
            .repositories
            .iter()
            .any(|r| r.id == version.repository)
        {
            return ready(Err(CatalogError::NotFound));
        }
        let siblings = || {
            state
                .versions
                .iter()
                .filter(|v| same_package(v, version.repository, &version.package))
        };
        if siblings().any(|v| v.version == version.version) {
            return ready(Err(CatalogError::VersionExists));
        }
        // Пакет хранит первое написание имени: берём его у любой уже опубликованной версии.
        let package = siblings()
            .next()
            .map_or(version.package.clone(), |v| v.package.clone());
        let record = PackageVersion {
            id: version.id,
            repository: version.repository,
            package,
            version: version.version,
            state: version.state,
            yanked: false,
            published_at: version.published_at,
            assets: version.assets,
            metadata: version.metadata,
        };
        state.versions.push(record.clone());
        ready(Ok(record))
    }

    fn versions(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<PackageVersion>, CatalogError>> + Send {
        let found = self
            .lock()
            .versions
            .iter()
            .filter(|v| same_package(v, repository, package))
            .cloned()
            .collect();
        ready(Ok(found))
    }

    fn find_version(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        version: &Version,
    ) -> impl Future<Output = Result<Option<PackageVersion>, CatalogError>> + Send {
        let found = self
            .lock()
            .versions
            .iter()
            .find(|v| same_package(v, repository, package) && v.version == *version)
            .cloned();
        ready(Ok(found))
    }

    fn set_yanked(
        &self,
        id: VersionId,
        yanked: bool,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        ready(self.update(id, |record| {
            record.yanked = yanked;
            Ok(())
        }))
    }

    fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        ready(self.update(id, |record| {
            if record.state != from {
                return Err(CatalogError::StateConflict {
                    actual: record.state,
                });
            }
            record.state = to;
            Ok(())
        }))
    }
}
