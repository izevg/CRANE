//! Общие фикстуры: мир на «голых» in-memory адаптерах и декораторы для сценариев отказов.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "dead_code: фикстуры общие для трёх тестовых бинарей, каждый берёт своё подмножество; unwrap в тестах = падение теста"
)]

pub(crate) mod proxy;

use std::future::{Future, ready};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_app::registry::{Coordinate, Publish, Registry, Upload};
use crane_domain::access::{Caller, Principal, PrincipalKind, RequestContext, RequestId};
use crane_domain::registry::{
    Asset, Format, FormatMetadata, PackageName, PackageVersion, Repository, RepositoryId,
    RepositoryKind, Sha256, Version, VersionId, VersionState,
};
use crane_memory::{ManualClock, MemoryAudit, MemoryBlobStore, MemoryCatalog, SequentialIdGen};
use crane_ports::registry::{
    BlobError, BlobStore, ByteStream, Catalog, CatalogError, Digests, NewVersion, Staged,
};
use crane_ports::system::IdGen;
use futures_executor::block_on;
use futures_util::StreamExt;
use futures_util::stream;

pub(crate) const HOSTED: &str = "crates";
pub(crate) const PROXY: &str = "mirror";
/// sha256 от байт `hello`.
const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
pub(crate) const METADATA: &str = r#"{"deps":[]}"#;

pub(crate) fn fixed_now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000)
}

pub(crate) fn principal() -> Principal {
    Principal {
        id: "tester".to_owned(),
        kind: PrincipalKind::Robot,
    }
}

pub(crate) fn request_context() -> RequestContext {
    RequestContext {
        request_id: RequestId::parse("req-1").unwrap(),
        trace_id: None,
        client_ip: None,
        user_agent: None,
    }
}

pub(crate) fn caller() -> Caller {
    Caller {
        principal: principal(),
        request: request_context(),
    }
}

pub(crate) fn hello_sha256() -> Sha256 {
    Sha256::from_hex(HELLO_SHA256).unwrap()
}

pub(crate) fn name(raw: &str) -> PackageName {
    PackageName::parse(Format::Cargo, raw).unwrap()
}

pub(crate) fn version(raw: &str) -> Version {
    Version::parse(Format::Cargo, raw).unwrap()
}

pub(crate) fn coordinate(package: &str, raw_version: &str) -> Coordinate {
    Coordinate {
        format: Format::Cargo,
        repository: HOSTED.to_owned(),
        package: name(package),
        version: version(raw_version),
    }
}

pub(crate) fn asset_path(package: &str, raw_version: &str) -> String {
    format!("{package}-{raw_version}.crate")
}

fn body(chunks: &[&'static [u8]]) -> ByteStream {
    let items: Vec<Result<Bytes, io::Error>> = chunks
        .iter()
        .map(|chunk| Ok(Bytes::from_static(chunk)))
        .collect();
    Box::pin(stream::iter(items))
}

pub(crate) fn publish_request(
    package: &str,
    raw_version: &str,
    chunks: &[&'static [u8]],
    declared_sha256: Option<Sha256>,
) -> Publish {
    Publish {
        coordinate: coordinate(package, raw_version),
        file: Upload {
            path: asset_path(package, raw_version),
            body: body(chunks),
            declared_sha256,
        },
        metadata: FormatMetadata::new(METADATA.to_owned()),
    }
}

pub(crate) fn read(stream: ByteStream) -> Vec<u8> {
    block_on(stream.collect::<Vec<_>>())
        .into_iter()
        .flat_map(|chunk| chunk.unwrap())
        .collect()
}

/// Мир, собранный до `Registry::new`: Registry владеет портами, достучаться до них потом нельзя.
pub(crate) struct Seed {
    pub(crate) catalog: MemoryCatalog,
    pub(crate) blobs: MemoryBlobStore,
    pub(crate) ids: Arc<SequentialIdGen>,
    audit: Arc<MemoryAudit>,
    pub(crate) hosted: RepositoryId,
}

impl Seed {
    pub(crate) fn new() -> Self {
        let ids = Arc::new(SequentialIdGen::default());
        let catalog = MemoryCatalog::default();
        let hosted = Self::ensure(&catalog, &ids, HOSTED, RepositoryKind::Hosted);
        Self {
            catalog,
            blobs: MemoryBlobStore::default(),
            ids,
            audit: Arc::default(),
            hosted,
        }
    }

    fn ensure(
        catalog: &MemoryCatalog,
        ids: &SequentialIdGen,
        repository: &str,
        kind: RepositoryKind,
    ) -> RepositoryId {
        block_on(catalog.ensure_repository(Repository {
            id: RepositoryId::new(ids.generate()),
            format: Format::Cargo,
            kind,
            name: repository.to_owned(),
        }))
        .unwrap()
        .id
    }

    /// Общий с `Registry` журнал аудита: после `registry()` самого `Seed` уже нет.
    pub(crate) fn audit(&self) -> Arc<MemoryAudit> {
        Arc::clone(&self.audit)
    }

    pub(crate) fn with_proxy(self) -> Self {
        Self::ensure(&self.catalog, &self.ids, PROXY, RepositoryKind::Proxy);
        self
    }

    /// Кладёт blob в CAS (stage + commit) и возвращает его Asset.
    fn put_blob(&self, path: &str, content: &'static [u8]) -> Asset {
        let staged = block_on(self.blobs.stage(body(&[content]))).unwrap();
        let digests = block_on(self.blobs.commit(staged)).unwrap();
        Asset {
            path: path.to_owned(),
            sha256: digests.sha256,
            size: digests.size,
        }
    }

    fn insert(
        &self,
        package: &str,
        raw_version: &str,
        state: VersionState,
        asset: Asset,
    ) -> PackageVersion {
        block_on(self.catalog.publish_version(NewVersion {
            id: VersionId::new(self.ids.generate()),
            repository: self.hosted,
            package: name(package),
            version: version(raw_version),
            state,
            published_at: fixed_now(),
            assets: vec![asset],
            metadata: FormatMetadata::new(METADATA.to_owned()),
        }))
        .unwrap()
    }

    pub(crate) fn yank(&self, record: &PackageVersion) {
        block_on(self.catalog.set_yanked(record.id, true)).unwrap();
    }

    /// Версия сразу в нужном состоянии, blob в CAS.
    pub(crate) fn add_version(
        &self,
        package: &str,
        raw_version: &str,
        state: VersionState,
        content: &'static [u8],
    ) -> PackageVersion {
        let asset = self.put_blob(&asset_path(package, raw_version), content);
        self.insert(package, raw_version, state, asset)
    }

    /// Запись каталога со ссылкой на blob, которого в CAS нет.
    pub(crate) fn add_dangling(&self, package: &str, raw_version: &str) -> PackageVersion {
        let asset = Asset {
            path: asset_path(package, raw_version),
            sha256: Sha256::from([0xee; 32]),
            size: 1,
        };
        self.insert(package, raw_version, VersionState::Released, asset)
    }

    pub(crate) fn registry_with<C: Catalog, B: BlobStore>(
        self,
        wrap_catalog: impl FnOnce(MemoryCatalog) -> C,
        wrap_blobs: impl FnOnce(MemoryBlobStore) -> B,
    ) -> Registry<C, B> {
        Registry::new(
            wrap_catalog(self.catalog),
            wrap_blobs(self.blobs),
            Arc::new(ManualClock::new(fixed_now())),
            self.ids,
            self.audit,
        )
    }

    pub(crate) fn registry(self) -> Registry<MemoryCatalog, MemoryBlobStore> {
        self.registry_with(|catalog| catalog, |blobs| blobs)
    }
}

/// Каталог с «слепой» предпроверкой: `find_version` всегда `None`, остальное делегируется.
/// Моделирует гонку: между предпроверкой и `publish_version` версию занял другой запрос.
#[derive(Debug)]
pub(crate) struct BlindCatalog(pub MemoryCatalog);

impl Catalog for BlindCatalog {
    fn ensure_repository(
        &self,
        repository: Repository,
    ) -> impl Future<Output = Result<Repository, CatalogError>> + Send {
        self.0.ensure_repository(repository)
    }

    fn find_repository(
        &self,
        format: Format,
        name: &str,
    ) -> impl Future<Output = Result<Option<Repository>, CatalogError>> + Send {
        self.0.find_repository(format, name)
    }

    fn publish_version(
        &self,
        version: NewVersion,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        self.0.publish_version(version)
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
        _repository: RepositoryId,
        _package: &PackageName,
        _version: &Version,
    ) -> impl Future<Output = Result<Option<PackageVersion>, CatalogError>> + Send {
        ready(Ok(None))
    }

    fn set_yanked(
        &self,
        id: VersionId,
        yanked: bool,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.0.set_yanked(id, yanked)
    }

    fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.0.transition(id, from, to)
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
        ready(Err(CatalogError::Backend("БД недоступна".into())))
    }

    fn publish_version(
        &self,
        version: NewVersion,
    ) -> impl Future<Output = Result<PackageVersion, CatalogError>> + Send {
        self.0.publish_version(version)
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

    fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.0.transition(id, from, to)
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
