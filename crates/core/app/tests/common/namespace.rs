//! Фикстуры namespace: ответы публичного upstream, сбои поиска имён, посев namespace.

use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crane_domain::registry::{
    Asset, Format, FormatMetadata, Namespace, NamespacePattern, PackageName, PackageVersion,
    Repository, RepositoryId, RepositoryKind, Sha256, Version, VersionId, VersionState,
    WorkspaceId,
};
use crane_memory::{MemoryCatalog, SequentialIdGen};
use crane_ports::proxy::{Presence, UpstreamNames};
use crane_ports::registry::{Catalog, CatalogError, NewVersion};
use crane_ports::system::IdGen;
use futures_executor::block_on;

use super::{METADATA, Seed, asset_path, fixed_now, name, version};

/// Workspace, отличный от `WorkspaceId::DEFAULT` (владельца репозиториев). uuid в зависимости
/// `app` нет, поэтому второй id берётся у детерминированного генератора: он даёт 1, затем 2.
pub(crate) fn other_workspace() -> WorkspaceId {
    let ids = SequentialIdGen::default();
    ids.generate();
    WorkspaceId::new(ids.generate())
}

/// Namespace формата cargo, уже разобранный.
pub(crate) fn cargo_namespace(pattern: &str, workspace: WorkspaceId) -> Namespace {
    Namespace {
        format: Format::Cargo,
        pattern: NamespacePattern::parse(Format::Cargo, pattern).unwrap(),
        workspace,
    }
}

impl Seed {
    pub(crate) fn with_upstream(mut self, upstream: Arc<dyn UpstreamNames>) -> Self {
        self.upstream = upstream;
        self
    }

    /// Резервирует namespace (формат cargo) за workspace напрямую в каталоге.
    pub(crate) fn add_namespace(&self, pattern: &str, workspace: WorkspaceId) -> Namespace {
        block_on(
            self.catalog
                .ensure_namespace(cargo_namespace(pattern, workspace)),
        )
        .unwrap()
    }

    /// Пакет в другом hosted-репозитории того же формата.
    pub(crate) fn add_package_in_other_hosted(&self, repository: &str, package: &str) {
        let other = Self::ensure(&self.catalog, &self.ids, repository, RepositoryKind::Hosted);
        let asset = self.put_blob(&asset_path(package, "1.0.0"), b"x");
        block_on(self.catalog.publish_version(NewVersion {
            id: VersionId::new(self.ids.generate()),
            repository: other,
            package: name(package),
            version: version("1.0.0"),
            state: VersionState::Released,
            published_at: fixed_now(),
            assets: vec![asset],
            metadata: FormatMetadata::new(METADATA.to_owned()),
            tags: Vec::new(),
        }))
        .unwrap();
    }
}

/// Публичный upstream с заданным ответом; считает вопросы, чтобы тест мог доказать,
/// что upstream не спрашивали.
#[derive(Debug)]
pub(crate) struct FakeUpstream {
    presence: Presence,
    queries: AtomicUsize,
}

impl FakeUpstream {
    pub(crate) fn new(presence: Presence) -> Arc<Self> {
        Arc::new(Self {
            presence,
            queries: AtomicUsize::new(0),
        })
    }

    pub(crate) fn queries(&self) -> usize {
        self.queries.load(Ordering::SeqCst)
    }
}

impl UpstreamNames for FakeUpstream {
    fn presence<'a>(
        &'a self,
        _format: Format,
        _name: &'a PackageName,
    ) -> Pin<Box<dyn Future<Output = Presence> + Send + 'a>> {
        self.queries.fetch_add(1, Ordering::SeqCst);
        Box::pin(ready(self.presence))
    }
}

/// Метод каталога, который `FaultyCatalog` заставляет отказывать.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fault {
    NamespaceFor,
    HostedExists,
    AddAsset,
    /// Параллельный запрос успел записать тот же файл: `add_asset` отвечает `AssetExists`.
    LoseAssetRaceToSameFile,
    /// То же, но победитель записал под этим путём другое содержимое.
    LoseAssetRaceToOtherFile,
}

/// Каталог, у которого отказывают перечисленные методы; остальное делегируется.
/// Моделирует сбой БД на пути проверки «внутреннее ли имя».
#[derive(Debug)]
pub(crate) struct FaultyCatalog {
    pub inner: MemoryCatalog,
    pub failing: &'static [Fault],
}

impl FaultyCatalog {
    fn fails(&self, fault: Fault) -> bool {
        self.failing.contains(&fault)
    }
}

fn lookup_down() -> CatalogError {
    CatalogError::Backend("БД недоступна".into())
}

impl Catalog for FaultyCatalog {
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

    async fn add_asset(
        &self,
        version: VersionId,
        asset: Asset,
    ) -> Result<PackageVersion, CatalogError> {
        if self.fails(Fault::AddAsset) {
            return Err(lookup_down());
        }
        let winner = if self.fails(Fault::LoseAssetRaceToSameFile) {
            Some(asset.clone())
        } else if self.fails(Fault::LoseAssetRaceToOtherFile) {
            Some(Asset {
                sha256: Sha256::from([0xee; 32]),
                ..asset.clone()
            })
        } else {
            None
        };
        if let Some(winner) = winner {
            self.inner.add_asset(version, winner).await?;
            return Err(CatalogError::AssetExists);
        }
        self.inner.add_asset(version, asset).await
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
        if self.fails(Fault::NamespaceFor) {
            return Err(lookup_down());
        }
        self.inner.namespace_for(format, name).await
    }

    async fn hosted_exists(
        &self,
        format: Format,
        name: &PackageName,
    ) -> Result<bool, CatalogError> {
        if self.fails(Fault::HostedExists) {
            return Err(lookup_down());
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
