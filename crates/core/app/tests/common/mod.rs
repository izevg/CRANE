//! Общие фикстуры: мир на «голых» in-memory адаптерах и декораторы для сценариев отказов.

#![allow(
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "фикстуры общие для нескольких тестовых бинарей и используются выборочно"
)]

use std::future::Future;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_app::registry::{Coordinate, Publish, Registry, Upload};
use crane_domain::registry::{
    Asset, Format, FormatMetadata, PackageName, PackageVersion, Repository, RepositoryId,
    RepositoryKind, Sha256, Version, VersionId, VersionState,
};
use crane_memory::{FixedClock, MemoryBlobStore, MemoryCatalog, SequentialIdGen};
use crane_ports::registry::{
    BlobError, BlobStore, ByteStream, Catalog, CatalogError, Digests, NewVersion, Staged,
};
use crane_ports::system::IdGen;
use futures_executor::block_on;
use futures_util::StreamExt;
use futures_util::stream;

pub const HOSTED: &str = "crates";
pub const PROXY: &str = "mirror";
/// sha256 от байт `hello`.
pub const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
pub const METADATA: &str = r#"{"deps":[]}"#;

pub fn fixed_now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000)
}

pub fn hello_sha256() -> Sha256 {
    Sha256::from_hex(HELLO_SHA256).unwrap()
}

pub fn name(raw: &str) -> PackageName {
    PackageName::parse(Format::Cargo, raw).unwrap()
}

pub fn version(raw: &str) -> Version {
    Version::parse(Format::Cargo, raw).unwrap()
}

pub fn coordinate(package: &str, raw_version: &str) -> Coordinate {
    Coordinate {
        format: Format::Cargo,
        repository: HOSTED.to_owned(),
        package: name(package),
        version: version(raw_version),
    }
}

pub fn asset_path(package: &str, raw_version: &str) -> String {
    format!("{package}-{raw_version}.crate")
}

pub fn body(chunks: &[&'static [u8]]) -> ByteStream {
    let items: Vec<Result<Bytes, io::Error>> = chunks
        .iter()
        .map(|chunk| Ok(Bytes::from_static(chunk)))
        .collect();
    Box::pin(stream::iter(items))
}

pub fn publish_request(
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

pub fn read(stream: ByteStream) -> Vec<u8> {
    block_on(stream.collect::<Vec<_>>())
        .into_iter()
        .flat_map(|chunk| chunk.unwrap())
        .collect()
}

pub fn assert_send<T: Send>(_: &T) {}

/// Мир, собранный до `Registry::new`: Registry владеет портами, достучаться до них потом нельзя.
pub struct Seed {
    pub catalog: MemoryCatalog,
    pub blobs: MemoryBlobStore,
    pub ids: Arc<SequentialIdGen>,
    pub hosted: RepositoryId,
}

impl Seed {
    pub fn new() -> Self {
        let ids = Arc::new(SequentialIdGen::default());
        let catalog = MemoryCatalog::default();
        let hosted = Self::ensure(&catalog, &ids, HOSTED, RepositoryKind::Hosted);
        Self {
            catalog,
            blobs: MemoryBlobStore::default(),
            ids,
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

    pub fn with_proxy(self) -> Self {
        Self::ensure(&self.catalog, &self.ids, PROXY, RepositoryKind::Proxy);
        self
    }

    /// Кладёт blob в CAS (stage + commit) и возвращает его Asset.
    pub fn put_blob(&self, path: &str, content: &'static [u8]) -> Asset {
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

    /// Версия сразу в нужном состоянии, blob в CAS.
    pub fn add_version(
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
    pub fn add_dangling(&self, package: &str, raw_version: &str) -> PackageVersion {
        let asset = Asset {
            path: asset_path(package, raw_version),
            sha256: Sha256::from([0xee; 32]),
            size: 1,
        };
        self.insert(package, raw_version, VersionState::Released, asset)
    }

    /// Публикует `Released` и затем переводит в `to`: так версия приходит в Deleted/Blocked.
    pub fn add_moved(
        &self,
        package: &str,
        raw_version: &str,
        to: VersionState,
        content: &'static [u8],
    ) -> PackageVersion {
        let record = self.add_version(package, raw_version, VersionState::Released, content);
        block_on(
            self.catalog
                .transition(record.id, VersionState::Released, to),
        )
        .unwrap();
        PackageVersion {
            state: to,
            ..record
        }
    }

    pub fn registry_with<C: Catalog, B: BlobStore>(
        self,
        wrap_catalog: impl FnOnce(MemoryCatalog) -> C,
        wrap_blobs: impl FnOnce(MemoryBlobStore) -> B,
    ) -> Registry<C, B> {
        Registry::new(
            wrap_catalog(self.catalog),
            wrap_blobs(self.blobs),
            Arc::new(FixedClock::new(fixed_now())),
            self.ids,
        )
    }

    pub fn registry(self) -> Registry<MemoryCatalog, MemoryBlobStore> {
        self.registry_with(|catalog| catalog, |blobs| blobs)
    }
}

/// Журнал вызовов `set_yanked`, общий с тестом.
pub type YankLog = Arc<Mutex<Vec<bool>>>;

pub fn yank_log() -> YankLog {
    Arc::default()
}

pub fn yank_calls(log: &YankLog) -> Vec<bool> {
    log.lock().unwrap().clone()
}

/// Каталог-декоратор над `MemoryCatalog`.
///
/// `blind`: `find_version` всегда `None`. Моделирует гонку: между предпроверкой
/// и `publish_version` версию занял другой запрос.
/// `spying`: записывает вызовы `set_yanked`, чтобы проверять, что запись не произошла.
#[derive(Debug)]
pub struct InstrumentedCatalog {
    inner: MemoryCatalog,
    blind: bool,
    yanks: YankLog,
}

impl InstrumentedCatalog {
    pub fn blind(inner: MemoryCatalog) -> Self {
        Self {
            inner,
            blind: true,
            yanks: yank_log(),
        }
    }

    pub fn spying(inner: MemoryCatalog, yanks: YankLog) -> Self {
        Self {
            inner,
            blind: false,
            yanks,
        }
    }
}

impl Catalog for InstrumentedCatalog {
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
        let blind = self.blind;
        let found = self.inner.find_version(repository, package, version);
        async move { if blind { Ok(None) } else { found.await } }
    }

    fn set_yanked(
        &self,
        id: VersionId,
        yanked: bool,
    ) -> impl Future<Output = Result<(), CatalogError>> + Send {
        self.yanks.lock().unwrap().push(yanked);
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
}

/// Журнал попыток commit, общий с тестом.
pub type CommitLog = Arc<Mutex<Vec<Sha256>>>;

/// Blob-хранилище, записывающее попытки commit; при `fail_commit` commit завершается ошибкой.
#[derive(Debug)]
pub struct RecordingBlobs {
    inner: MemoryBlobStore,
    commits: CommitLog,
    fail_commit: bool,
}

impl RecordingBlobs {
    pub fn new(inner: MemoryBlobStore, commits: CommitLog, fail_commit: bool) -> Self {
        Self {
            inner,
            commits,
            fail_commit,
        }
    }
}

pub fn commit_log() -> CommitLog {
    Arc::default()
}

pub fn committed(log: &CommitLog) -> Vec<Sha256> {
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
