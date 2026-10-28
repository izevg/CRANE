//! Общие фикстуры: мир на «голых» in-memory адаптерах и декораторы для сценариев отказов.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "dead_code: фикстуры общие для трёх тестовых бинарей, каждый берёт своё подмножество; unwrap в тестах = падение теста"
)]

pub(crate) mod faults;
pub(crate) mod namespace;
pub(crate) mod proxy;

use std::io;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_app::registry::{Coordinate, Publish, Registry, Upload};
use crane_domain::access::{Caller, Principal, PrincipalKind, RequestContext, RequestId};
use crane_domain::registry::{
    Asset, Checksum, Format, FormatMetadata, PackageName, PackageRef, PackageVersion, Repository,
    RepositoryId, RepositoryKind, Sha1, Sha256, Sha512, Version, VersionId, VersionState,
};
use crane_memory::{ManualClock, MemoryAudit, MemoryBlobStore, MemoryCatalog, SequentialIdGen};
use crane_ports::proxy::{NoUpstream, UpstreamNames};
use crane_ports::registry::{BlobStore, ByteStream, Catalog, NewVersion};
use crane_ports::system::IdGen;
use futures_executor::block_on;
use futures_util::StreamExt;
use futures_util::stream;

pub(crate) const HOSTED: &str = "crates";
pub(crate) const PROXY: &str = "mirror";
/// sha256 от байт `hello`.
const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
/// sha512 и sha1 от байт `hello`.
const HELLO_SHA512: &str = "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043";
const HELLO_SHA1: &str = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
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

pub(crate) fn hello_sha512() -> Sha512 {
    Sha512::from_hex(HELLO_SHA512).unwrap()
}

pub(crate) fn hello_sha1() -> Sha1 {
    Sha1::from_hex(HELLO_SHA1).unwrap()
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

pub(crate) fn coordinate_in(repository: &str, package: &str, raw_version: &str) -> Coordinate {
    Coordinate {
        repository: repository.to_owned(),
        ..coordinate(package, raw_version)
    }
}

pub(crate) fn package_ref(repository: &str, package: &str) -> PackageRef {
    PackageRef {
        format: Format::Cargo,
        repository: repository.to_owned(),
        package: name(package),
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
    declared: Vec<Checksum>,
) -> Publish {
    Publish {
        coordinate: coordinate(package, raw_version),
        file: Upload {
            path: asset_path(package, raw_version),
            body: body(chunks),
            declared,
        },
        metadata: Ok(FormatMetadata::new(METADATA.to_owned())),
        tags: Vec::new(),
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
    upstream: Arc<dyn UpstreamNames>,
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
            // По умолчанию публичного upstream нет: новые имена свободны.
            upstream: Arc::new(NoUpstream),
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
        published_at: SystemTime,
    ) -> PackageVersion {
        block_on(self.catalog.publish_version(NewVersion {
            id: VersionId::new(self.ids.generate()),
            repository: self.hosted,
            package: name(package),
            version: version(raw_version),
            state,
            published_at,
            assets: vec![asset],
            metadata: FormatMetadata::new(METADATA.to_owned()),
            tags: Vec::new(),
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
        self.add_version_at(package, raw_version, state, content, fixed_now())
    }

    /// То же, но с заданным временем публикации (для окна удаления).
    pub(crate) fn add_version_at(
        &self,
        package: &str,
        raw_version: &str,
        state: VersionState,
        content: &'static [u8],
        published_at: SystemTime,
    ) -> PackageVersion {
        let asset = self.put_blob(&asset_path(package, raw_version), content);
        self.insert(package, raw_version, state, asset, published_at)
    }

    /// Запись каталога со ссылкой на blob, которого в CAS нет.
    pub(crate) fn add_dangling(&self, package: &str, raw_version: &str) -> PackageVersion {
        let asset = Asset {
            path: asset_path(package, raw_version),
            sha256: Sha256::from([0xee; 32]),
            size: 1,
        };
        self.insert(
            package,
            raw_version,
            VersionState::Released,
            asset,
            fixed_now(),
        )
    }

    pub(crate) fn registry_with<C: Catalog, B: BlobStore>(
        self,
        wrap_catalog: impl FnOnce(MemoryCatalog) -> C,
        wrap_blobs: impl FnOnce(MemoryBlobStore) -> B,
    ) -> Registry<C, B> {
        self.registry_at_with(fixed_now(), wrap_catalog, wrap_blobs)
    }

    /// Реестр с часами, остановленными на `now`.
    pub(crate) fn registry_at(self, now: SystemTime) -> Registry<MemoryCatalog, MemoryBlobStore> {
        self.registry_at_with(now, |catalog| catalog, |blobs| blobs)
    }

    fn registry_at_with<C: Catalog, B: BlobStore>(
        self,
        now: SystemTime,
        wrap_catalog: impl FnOnce(MemoryCatalog) -> C,
        wrap_blobs: impl FnOnce(MemoryBlobStore) -> B,
    ) -> Registry<C, B> {
        Registry::new(
            wrap_catalog(self.catalog),
            wrap_blobs(self.blobs),
            Arc::new(ManualClock::new(now)),
            self.ids,
            self.audit,
            self.upstream,
        )
    }

    pub(crate) fn registry(self) -> Registry<MemoryCatalog, MemoryBlobStore> {
        self.registry_with(|catalog| catalog, |blobs| blobs)
    }
}
