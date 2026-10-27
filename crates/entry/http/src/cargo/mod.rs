//! Маршруты cargo sparse-реестра под `/cargo/{repo}`.

mod api;
mod auth;
mod index;

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware::from_fn_with_state;
use axum::routing::{delete, get, put};
use crane_app::registry::Registry;
use crane_ports::audit::AuditSink;
use crane_ports::registry::{BlobStore, Catalog};

use auth::Auth;
pub use auth::StaticToken;

use crate::context::{TrustedProxies, request_context};
use crate::error::{CargoError, not_found};

/// Точка монтирования cargo-маршрутов; от неё же строится база в `config.json`.
const CARGO_PREFIX: &str = "/cargo";

/// Настройки cargo-маршрутов.
#[derive(Debug, Clone)]
pub struct CargoSettings {
    /// Внешний адрес без завершающего '/': клиент получает его в `config.json`.
    pub public_url: String,
    /// Предел тела публикации: оно буферизуется в памяти целиком.
    pub max_publish_bytes: usize,
}

/// Общее состояние обработчиков.
struct Shared<C, B> {
    registry: Arc<Registry<C, B>>,
    settings: CargoSettings,
}

/// Корневой router API: cargo под токеном, всё остальное — problem+json 404.
pub fn api_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: Arc<Registry<C, B>>,
    cargo: CargoSettings,
    token: StaticToken,
    proxies: TrustedProxies,
    audit: Arc<dyn AuditSink>,
) -> Router {
    let max_publish_bytes = cargo.max_publish_bytes;
    let shared = Arc::new(Shared {
        registry,
        settings: cargo,
    });
    // `layer`, а не `route_layer`: 401 должен закрывать и неизвестные пути,
    // иначе без токена можно перебирать маршруты.
    let cargo_router = Router::new()
        .route("/index/config.json", get(index::config))
        .route("/index/{*path}", get(index::sparse))
        .route(
            "/api/v1/crates/new",
            put(api::publish).layer(DefaultBodyLimit::max(max_publish_bytes)),
        )
        .route(
            "/api/v1/crates/{name}/{version}/download",
            get(api::download),
        )
        .route("/api/v1/crates/{name}/{version}/yank", delete(api::yank))
        .route("/api/v1/crates/{name}/{version}/unyank", put(api::unyank))
        .route(
            "/api/v1/crates/{name}/owners",
            get(api::owners)
                .put(api::owners_unsupported)
                .delete(api::owners_unsupported),
        )
        .fallback(cargo_not_found)
        .with_state(shared)
        .layer(from_fn_with_state(
            Arc::new(Auth { token, audit }),
            auth::require_token,
        ));
    // Самый внешний слой: контекст запроса нужен и auth, и fallback, и рендеру ошибок.
    Router::new()
        .nest(&format!("{CARGO_PREFIX}/{{repo}}"), cargo_router)
        .fallback(not_found)
        .layer(from_fn_with_state(Arc::new(proxies), request_context))
}

async fn cargo_not_found() -> CargoError {
    CargoError::not_found()
}
