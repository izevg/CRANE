//! Proxy-репозитории: маршруты `/cargo/<имя>` отдают crates.io через движок с кэшем и проверкой sha256.

mod routes;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, put};
use bytes::Bytes;
use crane_app::proxy::{ProxyEngine, ProxyError, Served};
use crane_domain::proxy::{Origin, UpstreamPath};
use crane_domain::registry::PackageName;
use crane_format_cargo::{INDEX, index_path};
use crane_ports::proxy::{FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::BlobStore;

use super::{CARGO_PREFIX, CargoSettings, cargo_not_found};

/// Источники одного proxy-репозитория: индекс и архивы живут на разных хостах, но под одним
/// `RepositoryId` (ключи не пересекаются, см. `crane_format_cargo::INDEX`).
#[derive(Debug, Clone)]
pub struct CargoProxyOrigins {
    pub index: Origin,
    pub dl: Origin,
}

/// Состояние обработчиков одного proxy-репозитория.
struct Shared<U, PC, L, B> {
    engine: ProxyEngine<U, PC, L, B>,
    origins: CargoProxyOrigins,
    /// База, которую клиент получает в `config.json`: наши `dl` и `api`, а не crates.io.
    base: String,
}

impl<U, PC, L, B> Shared<U, PC, L, B>
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    /// Единственный путь к индексу: в upstream уходит путь, построенный из имени, а не пришедший в URL.
    async fn index(&self, package: &PackageName) -> Result<Served<Bytes>, ProxyError> {
        // Путь из `index_path` всегда проходит разбор; `NotFound` здесь недостижим и нужен лишь типу.
        let path = UpstreamPath::parse(&index_path(package)).map_err(|_| ProxyError::NotFound)?;
        self.engine
            .metadata(&self.origins.index, &path, &INDEX)
            .await
    }
}

/// Router proxy-репозитория без слоя аутентификации: его накладывает `api_router`.
/// `name` попадает в `config.json`, поэтому совпадает с именем, под которым router монтируется.
pub fn cargo_proxy_router<U, PC, L, B>(
    engine: ProxyEngine<U, PC, L, B>,
    origins: CargoProxyOrigins,
    settings: &CargoSettings,
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
        origins,
        base: format!("{}{CARGO_PREFIX}/{name}", settings.public_url),
    });
    Router::new()
        .route("/index/config.json", get(routes::config))
        .route("/index/{*path}", get(routes::sparse))
        .route(
            "/api/v1/crates/{name}/{version}/download",
            get(routes::download),
        )
        .route("/api/v1/crates/new", put(routes::not_hosted))
        .fallback(cargo_not_found)
        .with_state(shared)
}
