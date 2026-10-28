//! Фикстура proxy-репозитория: скриптованный upstream без сети и движок на memory-адаптерах.

use std::collections::VecDeque;
use std::future::{Future, ready};
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use axum::Router;
use bytes::Bytes;
use crane_app::proxy::{ProxyEngine, ProxyLimits};
use crane_domain::proxy::{EgressPolicy, EgressRefusal, Origin};
use crane_domain::registry::{Format, PackageName, RepositoryId};
use crane_format_cargo::{DL_HOST, INDEX_HOST};
use crane_format_npm::REGISTRY_HOST;
use crane_http::{
    CargoProxyOrigins, CargoSettings, EngineUpstreamNames, NpmSettings, cargo_proxy_router,
    npm_proxy_router,
};
use crane_memory::{
    ManualClock, MemoryBlobStore, MemoryFetchLeases, MemoryProxyCache, SequentialIdGen,
};
use crane_ports::proxy::{
    ETag, Presence, Upstream, UpstreamError, UpstreamNames, UpstreamRequest, UpstreamResponse,
};
use crane_ports::system::IdGen;
use futures_util::stream;
use tokio::runtime::Builder;

use super::{MAX_NPM_PUBLISH_BYTES, MAX_PUBLISH_BYTES, PUBLIC_URL, TestRegistry};

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
fn presence_of(
    names: &EngineUpstreamNames<FakeUpstream, MemoryProxyCache, MemoryFetchLeases, MemoryBlobStore>,
    format: Format,
    name: &str,
) -> Presence {
    let name = PackageName::parse(format, name).unwrap();
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(names.presence(format, &name))
}

/// Движок cargo proxy-репозитория, фейковый upstream и часы движка.
pub(crate) struct Proxy {
    engine: TestEngine,
    origins: CargoProxyOrigins,
    pub(crate) upstream: FakeUpstream,
    pub(crate) clock: Arc<ManualClock>,
}

impl Proxy {
    pub(crate) fn new() -> Self {
        let clock = Arc::new(ManualClock::new(SystemTime::UNIX_EPOCH));
        let upstream = FakeUpstream::default();
        let policy = EgressPolicy::new([INDEX_HOST, DL_HOST], false);
        let repository = RepositoryId::new(SequentialIdGen::default().generate());
        let origin = |host: &str| Origin {
            repository,
            base: policy.admit("https", host, None, "").unwrap(),
        };
        let origins = CargoProxyOrigins {
            index: origin(INDEX_HOST),
            dl: origin(DL_HOST),
        };
        Self {
            engine: engine(&upstream, &clock),
            origins,
            upstream,
            clock,
        }
    }

    /// Router над реестром фикстуры: защита имён видит те же namespaces и hosted-пакеты.
    pub(crate) fn router(&self, registry: Arc<TestRegistry>) -> Router {
        let settings = CargoSettings {
            public_url: PUBLIC_URL.to_owned(),
            max_publish_bytes: MAX_PUBLISH_BYTES,
        };
        cargo_proxy_router(
            self.engine.clone(),
            registry,
            self.origins.clone(),
            &settings,
            NAME,
        )
    }

    /// Что upstream знает об имени: тот же движок, что у маршрутов; источник только для cargo.
    pub(crate) fn presence(&self, format: Format, name: &str) -> Presence {
        let names =
            EngineUpstreamNames::new(self.engine.clone(), Some(self.origins.index.clone()), None);
        presence_of(&names, format, name)
    }

    pub(crate) fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }
}

/// Движок npm proxy-репозитория, фейковый upstream и часы движка.
pub(crate) struct NpmProxy {
    engine: TestEngine,
    origin: Origin,
    pub(crate) upstream: FakeUpstream,
    pub(crate) clock: Arc<ManualClock>,
}

impl NpmProxy {
    pub(crate) fn new() -> Self {
        let clock = Arc::new(ManualClock::new(SystemTime::UNIX_EPOCH));
        let upstream = FakeUpstream::default();
        let policy = EgressPolicy::new([REGISTRY_HOST], false);
        let origin = Origin {
            repository: RepositoryId::new(SequentialIdGen::default().generate()),
            base: policy.admit("https", REGISTRY_HOST, None, "").unwrap(),
        };
        Self {
            engine: engine(&upstream, &clock),
            origin,
            upstream,
            clock,
        }
    }

    pub(crate) fn router(&self, registry: Arc<TestRegistry>) -> Router {
        let settings = NpmSettings {
            public_url: PUBLIC_URL.to_owned(),
            max_publish_bytes: MAX_NPM_PUBLISH_BYTES,
        };
        npm_proxy_router(
            self.engine.clone(),
            registry,
            self.origin.clone(),
            &settings,
            NPM_NAME,
        )
    }

    /// Что upstream знает об имени; источник только для npm.
    pub(crate) fn presence(&self, format: Format, name: &str) -> Presence {
        let names = EngineUpstreamNames::new(self.engine.clone(), None, Some(self.origin.clone()));
        presence_of(&names, format, name)
    }

    pub(crate) fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }
}
