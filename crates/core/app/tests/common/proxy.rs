//! Фикстуры proxy-движка: скриптованный upstream с журналом и гейтом, общие между «репликами» порты.

use std::collections::VecDeque;
use std::future::{Future, ready};
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_app::proxy::{ProxyEngine, ProxyLimits, Served};
use crane_domain::proxy::{
    EgressPolicy, EgressRefusal, MetadataPolicy, NegativePolicy, Origin, ProxyKey, UpstreamPath,
};
use crane_domain::registry::{RepositoryId, Sha256};
use crane_memory::{
    ManualClock, MemoryBlobStore, MemoryFetchLeases, MemoryProxyCache, SequentialIdGen,
};
use crane_ports::proxy::{
    CacheError, Cached, Claim, ETag, FetchLeases, Lease, LeaseError, ProxyCache, Upstream,
    UpstreamError, UpstreamRequest, UpstreamResponse,
};
use crane_ports::registry::ByteStream;
use crane_ports::system::IdGen;
use futures_channel::oneshot;
use futures_util::stream;

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

/// Один шаг скрипта upstream.
#[derive(Debug, Clone)]
pub(crate) enum Step {
    NotModified,
    /// 304 с обновлённым `Cache-Control: max-age`.
    NotModifiedWithMaxAge(Duration),
    Found(&'static [u8], Option<&'static str>),
    /// 200 с объявленным `Cache-Control: max-age`.
    FoundWithMaxAge(&'static [u8], Option<&'static str>, Duration),
    /// 200, тело приходит несколькими чанками.
    FoundChunks(&'static [&'static [u8]]),
    NotFound,
    Refused(EgressRefusal),
    Unavailable,
    /// Тело оборвалось на чтении.
    BrokenBody,
}

/// Вызов upstream, как его увидел фейк.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) path: String,
    pub(crate) if_none_match: Option<String>,
}

#[derive(Debug, Default)]
struct UpstreamState {
    script: Mutex<VecDeque<Step>>,
    calls: Mutex<Vec<Call>>,
    gate: Mutex<Option<oneshot::Receiver<()>>>,
}

/// Открывает гейт: до этого первый `fetch` висит.
#[derive(Debug)]
pub(crate) struct Gate(oneshot::Sender<()>);

impl Gate {
    pub(crate) fn open(self) {
        let _ = self.0.send(());
    }
}

/// Upstream по сценарию: ответы по очереди, журнал вызовов, опциональный гейт на первом вызове.
#[derive(Debug, Clone)]
pub(crate) struct FakeUpstream(Arc<UpstreamState>);

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap()
}

impl FakeUpstream {
    pub(crate) fn new(script: impl IntoIterator<Item = Step>) -> Self {
        let state = UpstreamState::default();
        locked(&state.script).extend(script);
        Self(Arc::new(state))
    }

    pub(crate) fn gated(script: impl IntoIterator<Item = Step>) -> (Self, Gate) {
        let upstream = Self::new(script);
        let (sender, receiver) = oneshot::channel();
        *locked(&upstream.0.gate) = Some(receiver);
        (upstream, Gate(sender))
    }

    pub(crate) fn calls(&self) -> Vec<Call> {
        locked(&self.0.calls).clone()
    }

    pub(crate) fn call_count(&self) -> usize {
        locked(&self.0.calls).len()
    }
}

fn stream_of(chunks: Vec<Result<Bytes, io::Error>>) -> ByteStream {
    Box::pin(stream::iter(chunks))
}

impl Upstream for FakeUpstream {
    async fn fetch(&self, request: UpstreamRequest<'_>) -> Result<UpstreamResponse, UpstreamError> {
        locked(&self.0.calls).push(Call {
            path: request.path.as_str().to_owned(),
            if_none_match: request.if_none_match.map(|etag| etag.as_str().to_owned()),
        });
        let gate = locked(&self.0.gate).take();
        if let Some(gate) = gate {
            let _ = gate.await;
        }
        let step = locked(&self.0.script).pop_front();
        match step {
            Some(Step::NotModified) => Ok(UpstreamResponse::NotModified { max_age: None }),
            Some(Step::NotModifiedWithMaxAge(max_age)) => Ok(UpstreamResponse::NotModified {
                max_age: Some(max_age),
            }),
            Some(Step::Found(body, etag)) => Ok(UpstreamResponse::Found {
                body: stream_of(vec![Ok(Bytes::from_static(body))]),
                etag: etag.map(|etag| ETag::new(etag.to_owned())),
                max_age: None,
            }),
            Some(Step::FoundWithMaxAge(body, etag, max_age)) => Ok(UpstreamResponse::Found {
                body: stream_of(vec![Ok(Bytes::from_static(body))]),
                etag: etag.map(|etag| ETag::new(etag.to_owned())),
                max_age: Some(max_age),
            }),
            Some(Step::FoundChunks(chunks)) => Ok(UpstreamResponse::Found {
                body: stream_of(
                    chunks
                        .iter()
                        .map(|chunk| Ok(Bytes::from_static(chunk)))
                        .collect(),
                ),
                etag: None,
                max_age: None,
            }),
            Some(Step::NotFound) => Ok(UpstreamResponse::NotFound),
            Some(Step::Refused(refusal)) => Err(UpstreamError::Refused(refusal)),
            Some(Step::BrokenBody) => Ok(UpstreamResponse::Found {
                body: stream_of(vec![Err(io::Error::other("обрыв"))]),
                etag: None,
                max_age: None,
            }),
            Some(Step::Unavailable) | None => {
                Err(UpstreamError::Unavailable("upstream лежит".into()))
            }
        }
    }
}

/// Порты не реализованы для `Arc`: общий между «репликами» кэш оборачиваем.
#[derive(Debug, Clone, Default)]
pub(crate) struct SharedCache(pub(crate) Arc<MemoryProxyCache>);

impl ProxyCache for SharedCache {
    fn get(
        &self,
        key: &ProxyKey,
    ) -> impl Future<Output = Result<Option<Cached>, CacheError>> + Send {
        self.0.get(key)
    }

    fn put(
        &self,
        key: &ProxyKey,
        entry: Cached,
        keep_for: Duration,
    ) -> impl Future<Output = Result<(), CacheError>> + Send {
        self.0.put(key, entry, keep_for)
    }
}

/// Кэш, который всегда отказывает.
#[derive(Debug)]
pub(crate) struct BrokenCache;

impl ProxyCache for BrokenCache {
    fn get(
        &self,
        _key: &ProxyKey,
    ) -> impl Future<Output = Result<Option<Cached>, CacheError>> + Send {
        ready(Err(CacheError::Backend("кэш лежит".into())))
    }

    fn put(
        &self,
        _key: &ProxyKey,
        _entry: Cached,
        _keep_for: Duration,
    ) -> impl Future<Output = Result<(), CacheError>> + Send {
        ready(Err(CacheError::Backend("кэш лежит".into())))
    }
}

/// Общие leases; `down` моделирует недоступный координатор.
#[derive(Debug, Clone)]
pub(crate) struct SharedLeases {
    inner: Arc<MemoryFetchLeases>,
    down: bool,
}

impl SharedLeases {
    pub(crate) fn new(clock: Arc<ManualClock>) -> Self {
        Self {
            inner: Arc::new(MemoryFetchLeases::new(clock)),
            down: false,
        }
    }

    pub(crate) fn down(clock: Arc<ManualClock>) -> Self {
        Self {
            down: true,
            ..Self::new(clock)
        }
    }
}

impl FetchLeases for SharedLeases {
    async fn claim(
        &self,
        key: &ProxyKey,
        lease: Duration,
        patience: Duration,
    ) -> Result<Claim, LeaseError> {
        if self.down {
            return Err(LeaseError::Backend("координатор лежит".into()));
        }
        self.inner.claim(key, lease, patience).await
    }

    // Не `async fn`: обёртка обязана сохранить eager-снятие порта, иначе брошенный future ничего не снимет.
    fn release(&self, lease: Lease) -> impl Future<Output = Result<(), LeaseError>> + Send {
        self.inner.release(lease)
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
        expected: Option<Sha256>,
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
