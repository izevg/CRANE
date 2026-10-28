use std::future::ready;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crane_domain::registry::{
    Asset, Format, Namespace, NamespacePattern, PackageName, PackageVersion, Repository,
    RepositoryId, RepositoryKind, Version, VersionId, VersionState,
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
    namespaces: Vec<Namespace>,
    /// В порядке публикации, как того требует контракт `versions`.
    versions: Vec<PackageVersion>,
    tags: Vec<Tag>,
}

/// Dist-тег: пакет определяется парой (репозиторий, имя), как и в `same_package`.
#[derive(Debug)]
struct Tag {
    repository: RepositoryId,
    package: PackageName,
    name: String,
    version: VersionId,
}

impl State {
    /// Ставит тег, перенося его, если он уже стоит у пакета.
    fn put_tag(
        &mut self,
        repository: RepositoryId,
        package: &PackageName,
        name: &str,
        version: VersionId,
    ) {
        let existing = self
            .tags
            .iter_mut()
            .find(|t| t.repository == repository && t.package == *package && t.name == name);
        match existing {
            Some(tag) => tag.version = version,
            None => self.tags.push(Tag {
                repository,
                package: package.clone(),
                name: name.to_owned(),
                version,
            }),
        }
    }
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

impl MemoryCatalog {
    fn attach(&self, id: VersionId, asset: Asset) -> Result<PackageVersion, CatalogError> {
        let mut state = self.lock();
        let record = state
            .versions
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(CatalogError::NotFound)?;
        if record.assets.iter().any(|a| a.path == asset.path) {
            return Err(CatalogError::AssetExists);
        }
        record.assets.push(asset);
        record.assets = sorted_by_path(std::mem::take(&mut record.assets));
        Ok(record.clone())
    }
}

/// Порядок файлов задаёт контракт `find_version`: байтовый порядок путей, как `COLLATE "C"` в PG.
fn sorted_by_path(mut assets: Vec<Asset>) -> Vec<Asset> {
    assets.sort_by(|a, b| a.path.cmp(&b.path));
    assets
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
            deprecated: None,
            published_at: version.published_at,
            assets: sorted_by_path(version.assets),
            metadata: version.metadata,
        };
        for tag in &version.tags {
            state.put_tag(record.repository, &record.package, tag, record.id);
        }
        state.versions.push(record.clone());
        ready(Ok(record))
    }

    fn add_asset(
        &self,
        version: VersionId,
        asset: Asset,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        ready(self.attach(version, asset))
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

    fn set_deprecated(
        &self,
        id: VersionId,
        message: Option<String>,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        ready(self.update(id, |record| {
            record.deprecated = message;
            Ok(())
        }))
    }

    fn set_tag(
        &self,
        version: VersionId,
        tag: &str,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        let mut state = self.lock();
        let target = state
            .versions
            .iter()
            .find(|v| v.id == version)
            .map(|v| (v.repository, v.package.clone()));
        let done = target.map_or(Err(CatalogError::NotFound), |(repository, package)| {
            state.put_tag(repository, &package, tag, version);
            Ok(())
        });
        ready(done)
    }

    fn remove_tag(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        tag: &str,
    ) -> impl Future<Output = Result<bool, CatalogError>> + Send {
        let mut state = self.lock();
        let before = state.tags.len();
        state
            .tags
            .retain(|t| !(t.repository == repository && t.package == *package && t.name == tag));
        ready(Ok(state.tags.len() < before))
    }

    fn tags(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> impl Future<Output = Result<Vec<(String, VersionId)>, CatalogError>> + Send {
        let mut found: Vec<_> = self
            .lock()
            .tags
            .iter()
            .filter(|t| t.repository == repository && t.package == *package)
            .map(|t| (t.name.clone(), t.version))
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        ready(Ok(found))
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

    fn namespace_for(
        &self,
        format: Format,
        candidates: &[NamespacePattern],
    ) -> impl Future<Output = Result<Option<Namespace>, CatalogError>> + Send {
        let state = self.lock();
        // Порядок кандидатов задаёт специфичность: первый найденный и есть ответ.
        let found = candidates
            .iter()
            .find_map(|pattern| {
                state
                    .namespaces
                    .iter()
                    .find(|n| n.format == format && n.pattern == *pattern)
            })
            .cloned();
        ready(Ok(found))
    }

    fn hosted_exists(
        &self,
        format: Format,
        name: &PackageName,
    ) -> impl Future<Output = Result<bool, CatalogError>> + Send {
        let state = self.lock();
        // Пакет существует, пока есть хоть одна его версия: записи не удаляются.
        let exists = state.versions.iter().any(|v| {
            v.package == *name
                && state.repositories.iter().any(|r| {
                    r.id == v.repository && r.format == format && r.kind == RepositoryKind::Hosted
                })
        });
        ready(Ok(exists))
    }

    fn ensure_namespace(
        &self,
        namespace: Namespace,
    ) -> impl Future<Output = Result<Namespace, CatalogError>> + Send {
        let mut state = self.lock();
        let existing = state
            .namespaces
            .iter()
            .find(|n| n.format == namespace.format && n.pattern == namespace.pattern)
            .cloned();
        let stored = existing.unwrap_or_else(|| {
            state.namespaces.push(namespace.clone());
            namespace
        });
        ready(Ok(stored))
    }
}
