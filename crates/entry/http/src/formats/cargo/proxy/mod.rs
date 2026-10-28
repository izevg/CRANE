//! Proxy-репозитории: маршруты `/cargo/<имя>` отдают crates.io через движок с кэшем и проверкой sha256.

mod routes;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, put};
use bytes::Bytes;
use crane_app::proxy::{ProxyEngine, ProxyError, Served};
use crane_app::registry::Registry;
use crane_domain::access::Caller;
use crane_domain::proxy::{EgressRefusal, Origin, UpstreamPath};
use crane_domain::registry::{Format, PackageName};
use crane_format_cargo::{DL_HOST, INDEX, INDEX_HOST, index_path};
use crane_ports::proxy::{FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::{BlobStore, Catalog};

use super::cargo_not_found;
use crate::formats::{FormatSettings, ProxySource};
use crate::serve::Refusal;

/// Источники одного proxy-репозитория: индекс и архивы живут на разных хостах, но под одним
/// `RepositoryId` (ключи не пересекаются, см. `crane_format_cargo::INDEX`).
#[derive(Debug, Clone)]
struct CargoProxyOrigins {
    index: Origin,
    dl: Origin,
}

fn origins(source: &ProxySource<'_>) -> Result<CargoProxyOrigins, EgressRefusal> {
    let origin = |host: &str| -> Result<Origin, EgressRefusal> {
        Ok(Origin {
            repository: source.repository,
            base: source.policy.admit("https", host, None, "")?,
            accept: None,
        })
    };
    Ok(CargoProxyOrigins {
        index: origin(INDEX_HOST)?,
        dl: origin(DL_HOST)?,
    })
}

/// Источник документа для проверки присутствия имени: индекс.
pub(in crate::formats) fn presence_origin(
    source: &ProxySource<'_>,
) -> Result<Origin, EgressRefusal> {
    Ok(origins(source)?.index)
}

/// Состояние обработчиков одного proxy-репозитория. `U, PC, L, B` — порты движка,
/// `RC, RB` — порты `Registry`, чья защита имён стоит перед движком.
struct Shared<U, PC, L, B, RC, RB> {
    engine: ProxyEngine<U, PC, L, B>,
    registry: Arc<Registry<RC, RB>>,
    origins: CargoProxyOrigins,
    /// База, которую клиент получает в `config.json`: наши `dl` и `api`, а не crates.io.
    base: String,
}

/// Общее состояние за `State` обработчиков.
type Handle<U, PC, L, B, RC, RB> = Arc<Shared<U, PC, L, B, RC, RB>>;

impl<U, PC, L, B, RC, RB> Shared<U, PC, L, B, RC, RB>
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
    RC: Catalog,
    RB: BlobStore,
{
    /// Единственный путь к индексу (и для sparse, и для download): сначала защита внутренних
    /// имён, потом движок. Внутреннее имя не доходит ни до кэша, ни до upstream. В upstream уходит
    /// путь, построенный из имени, а не пришедший в URL.
    async fn index(
        &self,
        caller: &Caller,
        package: &PackageName,
    ) -> Result<Served<Bytes>, Refusal> {
        self.registry
            .admit_upstream(caller, Format::Cargo, package)
            .await
            .map_err(Refusal::Registry)?;
        // Путь из `index_path` всегда проходит разбор; `NotFound` здесь недостижим и нужен лишь типу.
        let path = UpstreamPath::parse(&index_path(package)).map_err(|_| ProxyError::NotFound)?;
        Ok(self
            .engine
            .metadata(&self.origins.index, &path, &INDEX)
            .await?)
    }
}

/// Router proxy-репозитория без слоя аутентификации: его накладывает `api_router`.
/// Имя источника попадает в `config.json`, поэтому совпадает с именем, под которым router монтируется.
pub(in crate::formats) fn proxy_router<U, PC, L, B, RC, RB>(
    engine: ProxyEngine<U, PC, L, B>,
    registry: Arc<Registry<RC, RB>>,
    source: &ProxySource<'_>,
    settings: &FormatSettings,
) -> Result<Router, EgressRefusal>
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
    RC: Catalog + 'static,
    RB: BlobStore + 'static,
{
    let shared = Arc::new(Shared {
        engine,
        registry,
        origins: origins(source)?,
        base: settings.repo_url(Format::Cargo, source.name),
    });
    Ok(Router::new()
        .route("/index/config.json", get(routes::config))
        .route("/index/{*path}", get(routes::sparse))
        .route(
            "/api/v1/crates/{name}/{version}/download",
            get(routes::download),
        )
        .route("/api/v1/crates/new", put(routes::not_hosted))
        .fallback(cargo_not_found)
        .with_state(shared))
}
