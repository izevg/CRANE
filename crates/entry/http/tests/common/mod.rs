//! Фикстура: `api_router` на in-memory адаптерах с известным токеном и hosted-репозиторием `internal`.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "dead_code: фикстуры общие для тестовых бинарей; unwrap в тестах = падение теста"
)]

pub(crate) mod capture;
pub(crate) mod cargo;
pub(crate) mod names;
pub(crate) mod npm;
pub(crate) mod proxy;
pub(crate) mod switch;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use bytes::Bytes;
use crane_app::registry::Registry;
use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::audit::AuditEvent;
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_http::{
    CargoSettings, FormatSettings, NpmSettings, ProxyRouters, StaticToken, TrustedProxies,
    admin_router, api_router,
};
use crane_memory::{ManualClock, MemoryAudit, MemoryBlobStore, MemoryCatalog, SequentialIdGen};
use crane_ports::audit::AuditSink;
use crane_ports::proxy::{NoUpstream, UpstreamNames};
use crane_ports::registry::Catalog;
use crane_ports::system::IdGen;
use http_body_util::BodyExt;
use serde_json::Value;
use tokio::runtime::{Builder, Runtime};
use tower::ServiceExt;

use self::cargo::{index_path, publish_body};
use self::switch::SwitchCatalog;
/// Реестр фикстуры; proxy-router получает его для защиты имён.
pub(crate) type TestRegistry = Registry<SwitchCatalog, MemoryBlobStore>;

pub(crate) const TOKEN: &str = "s3cret-token";
pub(crate) const PUBLIC_URL: &str = "https://crane.example";
pub(crate) const MAX_PUBLISH_BYTES: usize = 4096;
/// Документ публикации npm несёт tarball в base64, поэтому предел шире.
pub(crate) const MAX_NPM_PUBLISH_BYTES: usize = 65536;
pub(crate) const REPO: &str = "internal";
/// Личность, которой принадлежит `TOKEN`.
pub(crate) const PRINCIPAL: &str = "crane";
const PROXY_REPO: &str = "mirror";
/// Hosted-репозиторий npm.
pub(crate) const NPM_REPO: &str = "web";
/// Имя proxy-репозитория npm в каталоге; routers монтируются снаружи.
pub(crate) const NPM_PROXY_REPO: &str = "npmjs";
const WAIT: std::time::Duration = std::time::Duration::from_secs(5);

pub(crate) struct Reply {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
}

impl Reply {
    pub(crate) fn text(&self) -> String {
        String::from_utf8(self.body.to_vec()).unwrap()
    }

    pub(crate) fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }

    pub(crate) fn header(&self, name: &str) -> &str {
        self.headers.get(name).unwrap().to_str().unwrap()
    }

    /// `detail` cargo-ошибки без суффикса `(request_id …)`; суффикс обязателен и равен заголовку.
    pub(crate) fn detail(&self) -> String {
        let json = self.json();
        let detail = json["errors"][0]["detail"].as_str().unwrap();
        let suffix = format!(" (request_id {})", self.header("x-request-id"));
        detail.strip_suffix(&suffix).unwrap().to_owned()
    }
}

/// Каталог с hosted- и proxy-репозиториями обоих форматов.
fn seeded_catalog(runtime: &Runtime, ids: &SequentialIdGen) -> MemoryCatalog {
    let catalog = MemoryCatalog::default();
    for (format, name, kind) in [
        (Format::Cargo, REPO, RepositoryKind::Hosted),
        (Format::Cargo, PROXY_REPO, RepositoryKind::Proxy),
        (Format::Npm, NPM_REPO, RepositoryKind::Hosted),
        (Format::Npm, NPM_PROXY_REPO, RepositoryKind::Proxy),
    ] {
        runtime
            .block_on(catalog.ensure_repository(Repository {
                id: RepositoryId::new(ids.generate()),
                format,
                kind,
                name: name.to_owned(),
            }))
            .unwrap();
    }
    catalog
}

pub(crate) struct Api {
    runtime: Runtime,
    router: Router,
    admin: Router,
    audit: Arc<MemoryAudit>,
    clock: Arc<ManualClock>,
    registry: Arc<TestRegistry>,
    ids: Arc<SequentialIdGen>,
    catalog_down: Arc<AtomicBool>,
}

impl Api {
    pub(crate) fn new() -> Self {
        Self::with_proxies("")
    }

    /// Доверенные прокси в формате `CRANE_TRUSTED_PROXIES`; события аудита копятся в памяти.
    pub(crate) fn with_proxies(proxies: &str) -> Self {
        let audit = Arc::new(MemoryAudit::default());
        Self::build(proxies, audit.clone(), audit)
    }

    /// `sink` получает события и use cases, и слоя auth; `audit` — то, что отдаёт `audit_events`.
    pub(crate) fn build(proxies: &str, sink: Arc<dyn AuditSink>, audit: Arc<MemoryAudit>) -> Self {
        Self::build_with(proxies, sink, audit, Arc::new(NoUpstream), |_| {
            ProxyRouters::default()
        })
    }

    /// Рядом с hosted-репозиториями монтируется cargo proxy-router над тем же `Registry`.
    pub(crate) fn with_proxy(proxy: &proxy::Proxy) -> Self {
        let audit = Arc::new(MemoryAudit::default());
        Self::build_with("", audit.clone(), audit, Arc::new(NoUpstream), |registry| {
            ProxyRouters {
                cargo: vec![(proxy::NAME.to_owned(), proxy.router(registry.clone()))],
                npm: Vec::new(),
            }
        })
    }

    /// Рядом с hosted-репозиториями монтируется npm proxy-router над тем же `Registry`.
    pub(crate) fn with_npm_proxy(proxy: &proxy::NpmProxy) -> Self {
        let audit = Arc::new(MemoryAudit::default());
        Self::build_with("", audit.clone(), audit, Arc::new(NoUpstream), |registry| {
            ProxyRouters {
                cargo: Vec::new(),
                npm: vec![(proxy::NPM_NAME.to_owned(), proxy.router(registry.clone()))],
            }
        })
    }

    /// Публичный реестр отвечает на вопрос «знаешь ли имя» заданным ответом.
    pub(crate) fn with_upstream(upstream: Arc<dyn UpstreamNames>) -> Self {
        let audit = Arc::new(MemoryAudit::default());
        Self::build_with("", audit.clone(), audit, upstream, |_| {
            ProxyRouters::default()
        })
    }

    fn build_with(
        proxies: &str,
        sink: Arc<dyn AuditSink>,
        audit: Arc<MemoryAudit>,
        upstream: Arc<dyn UpstreamNames>,
        mount: impl FnOnce(&Arc<TestRegistry>) -> ProxyRouters,
    ) -> Self {
        let runtime = Builder::new_current_thread().enable_all().build().unwrap();
        let ids = Arc::new(SequentialIdGen::default());
        let catalog = seeded_catalog(&runtime, &ids);
        let clock = Arc::new(ManualClock::new(SystemTime::UNIX_EPOCH));
        let catalog_down = Arc::new(AtomicBool::new(false));
        let registry = Arc::new(Registry::new(
            SwitchCatalog::new(catalog, catalog_down.clone()),
            MemoryBlobStore::default(),
            clock.clone(),
            ids.clone(),
            sink.clone(),
            upstream,
        ));
        let proxy_repos = mount(&registry);
        let router = api_router(
            registry.clone(),
            FormatSettings {
                cargo: CargoSettings {
                    public_url: PUBLIC_URL.to_owned(),
                    max_publish_bytes: MAX_PUBLISH_BYTES,
                },
                npm: NpmSettings {
                    public_url: PUBLIC_URL.to_owned(),
                    max_publish_bytes: MAX_NPM_PUBLISH_BYTES,
                },
            },
            StaticToken::new(
                TOKEN.as_bytes(),
                Principal {
                    id: PRINCIPAL.to_owned(),
                    kind: PrincipalKind::Robot,
                },
            ),
            TrustedProxies::parse(proxies).unwrap(),
            sink,
            proxy_repos,
        );
        Self {
            runtime,
            router,
            admin: admin_router(registry.clone()),
            audit,
            clock,
            registry,
            ids,
            catalog_down,
        }
    }

    /// Каталог перестаёт отвечать на поиск имён: так недоступная БД выглядит для защиты.
    pub(crate) fn catalog_down(&self) {
        self.catalog_down.store(true, Ordering::SeqCst);
    }

    /// Сдвигает часы реестра (окно снятия версий).
    pub(crate) fn advance(&self, by: std::time::Duration) {
        self.clock.advance(by);
    }

    /// Даёт задачам, запущенным запросом (фоновая ревалидация), выполниться, пока условие не истинно.
    pub(crate) fn wait_until(&self, condition: impl Fn() -> bool) {
        self.runtime.block_on(async {
            tokio::time::timeout(WAIT, async {
                while !condition() {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        });
    }

    pub(crate) fn audit_events(&self) -> Vec<AuditEvent> {
        self.audit.events()
    }

    /// Отправляет готовый запрос в `api_router`.
    pub(crate) fn send(&self, request: Request<Body>) -> Reply {
        self.call(&self.router, request)
    }

    fn call(&self, router: &Router, request: Request<Body>) -> Reply {
        self.runtime.block_on(async {
            let response = router.clone().oneshot(request).await.unwrap();
            let (parts, body) = response.into_parts();
            Reply {
                status: parts.status,
                headers: parts.headers,
                body: body.collect().await.unwrap().to_bytes(),
            }
        })
    }

    /// Запрос с верным токеном.
    pub(crate) fn authed(&self, method: Method, uri: &str, body: Vec<u8>) -> Reply {
        self.with_headers(
            method,
            uri,
            &[(header::AUTHORIZATION.as_str(), TOKEN)],
            body,
        )
    }

    pub(crate) fn get(&self, uri: &str) -> Reply {
        self.authed(Method::GET, uri, Vec::new())
    }

    pub(crate) fn with_headers(
        &self,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Reply {
        self.send(request(method, uri, headers, body, None))
    }

    /// Запрос с адресом пира, как его видит сервер через `ConnectInfo`.
    pub(crate) fn get_from(&self, peer: &str, uri: &str, headers: &[(&str, &str)]) -> Reply {
        let peer = peer.parse().unwrap();
        self.send(request(Method::GET, uri, headers, Vec::new(), Some(peer)))
    }

    pub(crate) fn admin_get(&self, uri: &str) -> Reply {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        self.call(&self.admin, request)
    }

    pub(crate) fn publish(&self, repo: &str, name: &str, vers: &str, krate: &[u8]) -> Reply {
        self.authed(
            Method::PUT,
            &format!("/cargo/{repo}/api/v1/crates/new"),
            publish_body(name, vers, krate),
        )
    }

    pub(crate) fn index_text(&self, name: &str) -> Reply {
        self.get(&format!("/cargo/{REPO}/index/{}", index_path(name)))
    }
}

/// Собирает запрос; `peer` кладётся в расширения так же, как это делает `axum::serve`.
pub(crate) fn request(
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    body: Vec<u8>,
    peer: Option<SocketAddr>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::from(body)).unwrap();
    if let Some(peer) = peer {
        request.extensions_mut().insert(ConnectInfo(peer));
    }
    request
}
