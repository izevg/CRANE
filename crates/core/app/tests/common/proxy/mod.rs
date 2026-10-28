//! Фикстуры proxy-движка: скриптованный upstream с журналом и гейтом, общие между «репликами» порты.

pub(crate) mod ports;
pub(crate) mod upstream;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_app::proxy::{ProxyEngine, ProxyLimits, Served};
use crane_domain::proxy::{
    EgressPolicy, MetadataPolicy, NegativePolicy, Origin, ProxyKey, UpstreamPath,
};
use crane_domain::registry::{Checksum, RepositoryId};
use crane_memory::{ManualClock, MemoryBlobStore, SequentialIdGen};
use crane_ports::system::IdGen;

use self::ports::{SharedCache, SharedLeases};
use self::upstream::{FakeUpstream, Step};
use super::read;

pub(crate) const TTL: Duration = Duration::from_secs(100);

pub(crate) const fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// Миллисекунда до границы: проверка «ещё не истекло» на самом краю.
pub(crate) fn just_before(boundary: Duration) -> Duration {
    boundary.checked_sub(Duration::from_millis(1)).unwrap()
}

pub(crate) fn epoch() -> SystemTime {
    SystemTime::UNIX_EPOCH + secs(1_000_000)
}

pub(crate) fn origin() -> Origin {
    let base = EgressPolicy::new(["index.crates.io"], false)
        .admit("https", "index.crates.io", None, "")
        .unwrap();
    Origin {
        repository: RepositoryId::new(SequentialIdGen::default().generate()),
        base,
        accept: None,
    }
}

/// Тот же источник, но с запросом конкретного представления документа.
pub(crate) fn with_accept(origin: &Origin, accept: &'static str) -> Origin {
    Origin {
        accept: Some(accept),
        ..origin.clone()
    }
}

pub(crate) fn path(raw: &str) -> UpstreamPath {
    UpstreamPath::parse(raw).unwrap()
}

pub(crate) fn key(raw: &str) -> ProxyKey {
    origin().key(&path(raw))
}

/// ttl 100 с; Fresh до 90 с, Revalidate до 200 с, ошибка прощается до 1100 с; adaptive: каждые 3 необновления.
pub(crate) fn policy() -> MetadataPolicy {
    MetadataPolicy {
        ttl: TTL,
        revalidate_window: TTL,
        stale_if_error: secs(1000),
        adaptive_after: std::num::NonZeroU32::new(3).unwrap(),
        negative: negative(),
    }
}

/// 404 хранится 60 с, дальше удваивается до 600 с.
pub(crate) const fn negative() -> NegativePolicy {
    NegativePolicy {
        ttl: secs(60),
        max: secs(600),
    }
}

/// Общее blob-хранилище: Registry и движок делят один CAS через `Arc`.
pub(crate) type SharedBlobs = Arc<MemoryBlobStore>;

pub(crate) type Engine = ProxyEngine<FakeUpstream, SharedCache, SharedLeases, SharedBlobs>;

/// Мир одного «процесса»: движок, общие порты и часы. `replica` даёт второй движок на тех же кэше и leases.
#[derive(Debug)]
pub(crate) struct World {
    pub(crate) engine: Engine,
    pub(crate) clock: Arc<ManualClock>,
    pub(crate) upstream: FakeUpstream,
    pub(crate) cache: SharedCache,
    pub(crate) leases: SharedLeases,
    pub(crate) blobs: SharedBlobs,
    pub(crate) origin: Origin,
}

impl World {
    pub(crate) fn new(script: impl IntoIterator<Item = Step>) -> Self {
        Self::build(FakeUpstream::new(script), false, ProxyLimits::DEFAULT)
    }

    pub(crate) fn with_upstream(upstream: FakeUpstream) -> Self {
        Self::build(upstream, false, ProxyLimits::DEFAULT)
    }

    /// Координатор недоступен: схлопывает только singleflight в процессе.
    pub(crate) fn without_leases(upstream: FakeUpstream) -> Self {
        Self::build(upstream, true, ProxyLimits::DEFAULT)
    }

    pub(crate) fn with_limits(script: impl IntoIterator<Item = Step>, limits: ProxyLimits) -> Self {
        Self::build(FakeUpstream::new(script), false, limits)
    }

    fn build(upstream: FakeUpstream, leases_down: bool, limits: ProxyLimits) -> Self {
        let clock = Arc::new(ManualClock::new(epoch()));
        let leases = if leases_down {
            SharedLeases::down(Arc::clone(&clock))
        } else {
            SharedLeases::new(Arc::clone(&clock))
        };
        let cache = SharedCache::default();
        let blobs = SharedBlobs::default();
        let engine = ProxyEngine::new(
            upstream.clone(),
            cache.clone(),
            leases.clone(),
            blobs.clone(),
            clock.clone(),
            limits,
        );
        Self {
            engine,
            clock,
            upstream,
            cache,
            leases,
            blobs,
            origin: origin(),
        }
    }

    /// Второй движок («реплика») на общих кэше, leases, blob и часах.
    pub(crate) fn replica(&self, upstream: FakeUpstream) -> Engine {
        self.replica_with_blobs(upstream, self.blobs.clone())
    }

    pub(crate) fn replica_with_blobs(&self, upstream: FakeUpstream, blobs: SharedBlobs) -> Engine {
        ProxyEngine::new(
            upstream,
            self.cache.clone(),
            self.leases.clone(),
            blobs,
            self.clock.clone(),
            ProxyLimits::DEFAULT,
        )
    }

    pub(crate) fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }

    pub(crate) fn metadata(
        &self,
        raw_path: &str,
    ) -> Result<Served<Bytes>, crane_app::proxy::ProxyError> {
        futures_executor::block_on(
            self.engine
                .metadata(&self.origin, &path(raw_path), &policy()),
        )
    }

    /// Документ через другой источник (то же хранилище, другое представление).
    pub(crate) fn metadata_via(
        &self,
        origin: &Origin,
        raw_path: &str,
    ) -> Result<Served<Bytes>, crane_app::proxy::ProxyError> {
        futures_executor::block_on(self.engine.metadata(origin, &path(raw_path), &policy()))
    }

    /// Сдвигает часы, берёт устаревший документ и исполняет его фоновую ревалидацию.
    pub(crate) fn refresh_after(&self, by: Duration, raw_path: &str) {
        self.advance(by);
        let mut served = self.metadata(raw_path).unwrap();
        let revalidation = served.revalidation.take().unwrap();
        futures_executor::block_on(revalidation).unwrap();
    }

    pub(crate) fn artifact(
        &self,
        raw_path: &str,
        expected: Option<Checksum>,
    ) -> Result<Served<crane_app::proxy::Fetched>, crane_app::proxy::ProxyError> {
        futures_executor::block_on(self.engine.artifact(
            &self.origin,
            &path(raw_path),
            expected,
            &NegativePolicy::ARTIFACT,
        ))
    }
}

/// Один опрос future без исполнителя: доводит её до первой точки ожидания.
pub(crate) fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

/// Читает тело артефакта целиком.
pub(crate) fn bytes_of(fetched: crane_app::proxy::Fetched) -> Vec<u8> {
    read(fetched.body)
}
