//! Фикстура proxy-репозитория: скриптованный upstream без сети и движок на memory-адаптерах.

use std::collections::{BTreeMap, VecDeque};
use std::future::{Future, ready};
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use axum::Router;
use bytes::Bytes;
use crane_app::proxy::{ProxyEngine, ProxyLimits};
use crane_domain::proxy::{EgressPolicy, EgressRefusal};
use crane_domain::registry::{Format, RepositoryId};
use crane_format_cargo::{API_HOST, DL_HOST, INDEX_HOST};
use crane_format_npm::REGISTRY_HOST;
use crane_http::{EngineUpstreamNames, FormatSettings, ProxySource, presence_origin, proxy_router};
use crane_memory::{
    ManualClock, MemoryBlobStore, MemoryFetchLeases, MemoryProxyCache, SequentialIdGen,
};
use crane_ports::proxy::{
    ETag, Presence, Upstream, UpstreamError, UpstreamNames, UpstreamRequest, UpstreamResponse,
};
use crane_ports::system::IdGen;
use futures_util::stream;
use tokio::runtime::Builder;

use super::{MAX_PUBLISH_BYTES, PUBLIC_URL, TestRegistry, ecosystem};

/// Имя proxy-репозитория в URL.
pub(crate) const NAME: &str = "crates-io";
/// Имя npm proxy-репозитория в URL.
pub(crate) const NPM_NAME: &str = "npmjs";

/// Один ответ upstream на запрос конкретного пути.
pub(crate) enum Step {
    Found(Vec<u8>, Option<&'static str>),
    NotFound,
    Refused(EgressRefusal),
    Unavailable,
}

/// Upstream по сценарию «путь → ответ»: запись расходуется при использовании, неизвестный путь недоступен.
#[derive(Clone, Default)]
pub(crate) struct FakeUpstream(Arc<State>);

#[derive(Default)]
struct State {
    script: Mutex<VecDeque<(String, Step)>>,
    calls: Mutex<Vec<(String, Option<String>)>>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap()
}

impl FakeUpstream {
    pub(crate) fn will(&self, path: &str, step: Step) {
        locked(&self.0.script).push_back((path.to_owned(), step));
    }

    /// Пути запросов к upstream по порядку.
    pub(crate) fn paths(&self) -> Vec<String> {
        locked(&self.0.calls)
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// `If-None-Match` последнего запроса.
    pub(crate) fn last_condition(&self) -> Option<String> {
        locked(&self.0.calls)
            .last()
            .and_then(|(_, tag)| tag.clone())
    }

    pub(crate) fn call_count(&self) -> usize {
        locked(&self.0.calls).len()
    }
}

impl Upstream for FakeUpstream {
    fn fetch(
        &self,
        request: UpstreamRequest<'_>,
    ) -> impl Future<Output = Result<UpstreamResponse, UpstreamError>> + Send {
        let path = request.path.as_str().to_owned();
        locked(&self.0.calls).push((
            path.clone(),
            request.if_none_match.map(|tag| tag.as_str().to_owned()),
        ));
        let step = {
            let mut script = locked(&self.0.script);
            let position = script.iter().position(|(scripted, _)| *scripted == path);
            position
                .and_then(|index| script.remove(index))
                .map(|(_, step)| step)
        };
        ready(match step {
            Some(Step::Found(body, etag)) => {
                let chunk: Result<Bytes, io::Error> = Ok(Bytes::from(body));
                Ok(UpstreamResponse::Found {
                    body: Box::pin(stream::iter([chunk])),
                    etag: etag.map(|tag| ETag::new(tag.to_owned())),
                    max_age: None,
                })
            }
            Some(Step::NotFound) => Ok(UpstreamResponse::NotFound),
            Some(Step::Refused(refusal)) => Err(UpstreamError::Refused(refusal)),
            Some(Step::Unavailable) | None => {
                Err(UpstreamError::Unavailable("upstream лежит".into()))
            }
        })
    }
}

type TestEngine = ProxyEngine<FakeUpstream, MemoryProxyCache, MemoryFetchLeases, MemoryBlobStore>;

fn engine(upstream: &FakeUpstream, clock: &Arc<ManualClock>) -> TestEngine {
    ProxyEngine::new(
        upstream.clone(),
        MemoryProxyCache::default(),
        MemoryFetchLeases::new(clock.clone()),
        MemoryBlobStore::default(),
        clock.clone(),
        ProxyLimits::DEFAULT,
    )
}

/// Блокирующий ответ «знает ли публичный реестр имя» от настоящей реализации над фейковым upstream.
fn presence_of(names: &dyn UpstreamNames, format: Format, name: &str) -> Presence {
    let name = ecosystem(format).parse_name(name).unwrap();
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(names.presence(format, &name))
}

/// Общее двух фикстур: движок, политика egress, источник и таблица форматов в роли сервера.
struct Fixture {
    format: Format,
    name: &'static str,
    engine: TestEngine,
    policy: EgressPolicy,
    repository: RepositoryId,
    upstream: FakeUpstream,
    clock: Arc<ManualClock>,
}

impl Fixture {
    fn new(format: Format, name: &'static str, hosts: &[&str]) -> Self {
        let clock = Arc::new(ManualClock::new(SystemTime::UNIX_EPOCH));
        let upstream = FakeUpstream::default();
        Self {
            format,
            name,
            engine: engine(&upstream, &clock),
            policy: EgressPolicy::new(hosts.iter().copied(), false),
            repository: RepositoryId::new(SequentialIdGen::default().generate()),
            upstream,
            clock,
        }
    }

    fn source(&self) -> ProxySource<'_> {
        ProxySource {
            repository: self.repository,
            name: self.name,
            policy: &self.policy,
        }
    }

    fn router(&self, registry: Arc<TestRegistry>) -> Router {
        let settings = FormatSettings {
            public_url: PUBLIC_URL.to_owned(),
            max_publish_bytes: MAX_PUBLISH_BYTES,
        };
        proxy_router(
            self.format,
            self.engine.clone(),
            registry,
            &self.source(),
            &settings,
        )
        .unwrap()
        .unwrap()
    }

    fn names(&self) -> Arc<dyn UpstreamNames> {
        let origin = presence_origin(self.format, &self.source())
            .unwrap()
            .unwrap();
        Arc::new(EngineUpstreamNames::new(
            self.engine.clone(),
            BTreeMap::from([(self.format, origin)]),
        ))
    }

    /// Что upstream знает об имени: тот же движок, что у маршрутов; источник только у формата фикстуры.
    fn presence(&self, format: Format, name: &str) -> Presence {
        presence_of(self.names().as_ref(), format, name)
    }
}

/// Движок cargo proxy-репозитория, фейковый upstream и часы движка.
pub(crate) struct Proxy {
    fixture: Fixture,
    pub(crate) upstream: FakeUpstream,
    pub(crate) clock: Arc<ManualClock>,
}

impl Proxy {
    pub(crate) fn new() -> Self {
        let fixture = Fixture::new(Format::Cargo, NAME, &[INDEX_HOST, DL_HOST, API_HOST]);
        Self {
            upstream: fixture.upstream.clone(),
            clock: fixture.clock.clone(),
            fixture,
        }
    }

    /// Router над реестром фикстуры: защита имён видит те же namespaces и hosted-пакеты.
    pub(crate) fn router(&self, registry: Arc<TestRegistry>) -> Router {
        self.fixture.router(registry)
    }

    pub(crate) fn presence(&self, format: Format, name: &str) -> Presence {
        self.fixture.presence(format, name)
    }

    /// Публичный реестр для `Registry`: тот же движок и кэш, что у маршрутов proxy, как на сервере.
    pub(crate) fn names(&self) -> Arc<dyn UpstreamNames> {
        self.fixture.names()
    }

    pub(crate) fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }
}

/// Движок npm proxy-репозитория, фейковый upstream и часы движка.
pub(crate) struct NpmProxy {
    fixture: Fixture,
    pub(crate) upstream: FakeUpstream,
    pub(crate) clock: Arc<ManualClock>,
}

impl NpmProxy {
    pub(crate) fn new() -> Self {
        let fixture = Fixture::new(Format::Npm, NPM_NAME, &[REGISTRY_HOST]);
        Self {
            upstream: fixture.upstream.clone(),
            clock: fixture.clock.clone(),
            fixture,
        }
    }

    pub(crate) fn router(&self, registry: Arc<TestRegistry>) -> Router {
        self.fixture.router(registry)
    }

    pub(crate) fn presence(&self, format: Format, name: &str) -> Presence {
        self.fixture.presence(format, name)
    }

    pub(crate) fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }
}
