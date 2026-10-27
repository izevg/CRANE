use std::collections::HashMap;
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

/// Версии лежат в порядке публикации, как того требует контракт `versions`.
#[derive(Debug, Default)]
struct State {
    repositories: Vec<Repository>,
    versions: Vec<Stored>,
    /// Первое написание имени пакета: ключ — `(репозиторий, нормализованное имя)`.
    written_names: HashMap<(RepositoryId, String), PackageName>,
}

#[derive(Debug)]
struct Stored {
    record: PackageVersion,
    canonical: String,
}

impl MemoryCatalog {
    // Замок берётся только в синхронных функциях: guard не должен жить через await.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn ensure_repository_sync(&self, repository: Repository) -> Repository {
        let mut state = self.lock();
        let existing = state
            .repositories
            .iter()
            .find(|r| r.format == repository.format && r.name == repository.name);
        if let Some(existing) = existing {
            return existing.clone();
        }
        state.repositories.push(repository.clone());
        repository
    }

    fn find_repository_sync(&self, format: Format, name: &str) -> Option<Repository> {
        let state = self.lock();
        state
            .repositories
            .iter()
            .find(|r| r.format == format && r.name == name)
            .cloned()
    }

    fn publish_sync(&self, new: NewVersion) -> Result<PackageVersion, CatalogError> {
        let mut state = self.lock();
        if !state.repositories.iter().any(|r| r.id == new.repository) {
            return Err(CatalogError::NotFound);
        }
        let canonical = new.version.canonical();
        let taken = state.versions.iter().any(|s| {
            s.record.repository == new.repository
                && s.record.package.normalized() == new.package.normalized()
                && s.canonical == canonical
        });
        if taken {
            return Err(CatalogError::VersionExists);
        }
        let package = state
            .written_names
            .entry((new.repository, new.package.normalized().to_owned()))
            .or_insert(new.package)
            .clone();
        let record = PackageVersion {
            id: new.id,
            repository: new.repository,
            package,
            version: new.version,
            state: new.state,
            yanked: false,
            published_at: new.published_at,
            assets: new.assets,
            metadata: new.metadata,
        };
        state.versions.push(Stored {
            record: record.clone(),
            canonical,
        });
        Ok(record)
    }

    fn versions_sync(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> Vec<PackageVersion> {
        let state = self.lock();
        state
            .versions
            .iter()
            .filter(|s| {
                s.record.repository == repository
                    && s.record.package.normalized() == package.normalized()
            })
            .map(|s| s.record.clone())
            .collect()
    }

    fn update(
        &self,
        id: VersionId,
        change: impl FnOnce(&mut PackageVersion) -> Result<(), CatalogError>,
    ) -> Result<(), CatalogError> {
        let mut state = self.lock();
        let stored = state
            .versions
            .iter_mut()
            .find(|s| s.record.id == id)
            .ok_or(CatalogError::NotFound)?;
        change(&mut stored.record)
    }
}

// Внутри нет `.await`, а `async fn` без него не проходит `clippy::unused_async`,
// поэтому методы возвращают готовые футуры.
impl Catalog for MemoryCatalog {
    fn ensure_repository(
        &self,
        repository: Repository,
    ) -> impl Future<Output = Result<Repository, CatalogError>> + Send {
        ready(Ok(self.ensure_repository_sync(repository)))
    }

    fn find_repository(
        &self,
        format: Format,
        name: &str,
    ) -> impl Future<Output = Result<Option<Repository>, CatalogError>> + Send {
        ready(Ok(self.find_repository_sync(format, name)))
    }

    fn publish_version(
        &self,
        version: NewVersion,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        ready(self.publish_sync(version))
    }

    fn versions(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<PackageVersion>, CatalogError>> + Send {
        ready(Ok(self.versions_sync(repository, package)))
    }

    fn find_version(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        version: &Version,
    ) -> impl Future<Output = Result<Option<PackageVersion>, CatalogError>> + Send {
        let found = self
            .versions_sync(repository, package)
            .into_iter()
            .find(|v| v.version == *version);
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
