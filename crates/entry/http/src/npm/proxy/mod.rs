//! Proxy-репозитории: `/npm/<имя>` отдаёт registry.npmjs.org через движок с кэшем и проверкой sha512.

mod routes;

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::any;
use bytes::Bytes;
use crane_app::proxy::{ProxyEngine, ProxyError, Served};
use crane_domain::proxy::Origin;
use crane_domain::registry::PackageName;
use crane_format_npm::{PACKUMENT, packument_path};
use crane_ports::proxy::{FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::BlobStore;

use super::{NpmSettings, npm_not_found};

/// Состояние обработчика одного proxy-репозитория.
struct Shared<U, PC, L, B> {
    engine: ProxyEngine<U, PC, L, B>,
    origin: Origin,
    /// База ссылок на tarball: наша точка монтирования, а не registry.npmjs.org.
    base: String,
}

impl<U, PC, L, B> Shared<U, PC, L, B>
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    /// Единственный путь к документу upstream: в сеть уходит путь, построенный из имени, а не пришедший
    /// в URL. Проверку «имя принадлежит hosted или namespace» (п. 6 дорожной карты) нужно ставить сюда.
    async fn packument(&self, package: &PackageName) -> Result<Served<Bytes>, ProxyError> {
        // Путь из имени, прошедшего грамматику, всегда разбирается; `NotFound` недостижим и нужен лишь типу.
        let path = packument_path(package).map_err(|_| ProxyError::NotFound)?;
        self.engine.metadata(&self.origin, &path, &PACKUMENT).await
    }
}

/// Router proxy-репозитория без слоёв аутентификации и диалекта: их накладывает `api_router`.
/// `name` попадает в ссылки на tarball, поэтому совпадает с именем, под которым router монтируется.
pub fn npm_proxy_router<U, PC, L, B>(
    engine: ProxyEngine<U, PC, L, B>,
    origin: Origin,
    settings: &NpmSettings,
    name: &str,
) -> Router
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    let shared = Arc::new(Shared {
        engine,
        origin,
        base: settings.mount(name),
    });
    Router::new()
        .route("/{*path}", any(routes::handle::<U, PC, L, B>))
        // Proxy тела не читает: запись в него всё равно закрыта (405).
        .layer(DefaultBodyLimit::max(0))
        .fallback(npm_not_found)
        .with_state(shared)
}
