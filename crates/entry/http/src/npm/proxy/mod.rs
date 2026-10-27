//! Proxy-репозитории: `/npm/<имя>` отдаёт registry.npmjs.org через движок с кэшем и проверкой sha512.

mod routes;

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::any;
use bytes::Bytes;
use crane_app::proxy::{ProxyEngine, ProxyError, Served};
use crane_app::registry::Registry;
use crane_domain::access::Caller;
use crane_domain::proxy::Origin;
use crane_domain::registry::{Format, PackageName};
use crane_format_npm::{PACKUMENT, packument_path};
use crane_ports::proxy::{FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::{BlobStore, Catalog};

use super::{NpmSettings, npm_not_found};
use crate::serve::Refusal;

/// Состояние обработчика одного proxy-репозитория. `U, PC, L, B` — порты движка,
/// `RC, RB` — порты `Registry`, чья защита имён стоит перед движком.
struct Shared<U, PC, L, B, RC, RB> {
    engine: ProxyEngine<U, PC, L, B>,
    registry: Arc<Registry<RC, RB>>,
    origin: Origin,
    /// База ссылок на tarball: наша точка монтирования, а не registry.npmjs.org.
    base: String,
}

/// Общее состояние за `State` обработчика.
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
    /// Единственный путь к документу upstream (и для packument, и для tarball): сначала защита
    /// внутренних имён, потом движок. Внутреннее имя не доходит ни до кэша, ни до upstream. В сеть
    /// уходит путь, построенный из имени, а не пришедший в URL.
    async fn packument(
        &self,
        caller: &Caller,
        package: &PackageName,
    ) -> Result<Served<Bytes>, Refusal> {
        self.registry
            .admit_upstream(caller, Format::Npm, package)
            .await
            .map_err(Refusal::Registry)?;
        // Путь из имени, прошедшего грамматику, всегда разбирается; `NotFound` недостижим и нужен лишь типу.
        let path = packument_path(package).map_err(|_| ProxyError::NotFound)?;
        Ok(self
            .engine
            .metadata(&self.origin, &path, &PACKUMENT)
            .await?)
    }
}

/// Router proxy-репозитория без слоёв аутентификации и диалекта: их накладывает `api_router`.
/// `name` попадает в ссылки на tarball, поэтому совпадает с именем, под которым router монтируется.
pub fn npm_proxy_router<U, PC, L, B, RC, RB>(
    engine: ProxyEngine<U, PC, L, B>,
    registry: Arc<Registry<RC, RB>>,
    origin: Origin,
    settings: &NpmSettings,
    name: &str,
) -> Router
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
        origin,
        base: settings.mount(name),
    });
    Router::new()
        .route("/{*path}", any(routes::handle::<U, PC, L, B, RC, RB>))
        // Proxy тела не читает: запись в него всё равно закрыта (405).
        .layer(DefaultBodyLimit::max(0))
        .fallback(npm_not_found)
        .with_state(shared)
}
