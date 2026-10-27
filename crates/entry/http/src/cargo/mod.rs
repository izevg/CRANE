//! Маршруты cargo sparse-реестра под `/cargo/{repo}`: hosted и proxy-репозитории.

mod hosted;
mod proxy;

use std::sync::Arc;

use axum::Router;
use axum::middleware::from_fn_with_state;
use crane_app::registry::Registry;
use crane_ports::audit::AuditSink;
use crane_ports::registry::{BlobStore, Catalog};

pub use proxy::{CargoProxyOrigins, cargo_proxy_router};

use crate::auth::{Auth, StaticToken, require_token};
use crate::context::{TrustedProxies, request_context};
use crate::error::{ClientError, Dialect, mark_dialect, not_found};

/// Точка монтирования cargo-маршрутов; от неё же строится база в `config.json`.
const CARGO_PREFIX: &str = "/cargo";
/// Тип тела sparse-индекса.
const INDEX_TYPE: &str = "text/plain; charset=utf-8";

/// Настройки cargo-маршрутов.
#[derive(Debug, Clone)]
pub struct CargoSettings {
    /// Внешний адрес без завершающего '/': клиент получает его в `config.json`.
    pub public_url: String,
    /// Предел тела публикации: оно буферизуется в памяти целиком.
    pub max_publish_bytes: usize,
}

/// Корневой router API: cargo под токеном, всё остальное — problem+json 404.
///
/// `proxy_repos` — готовые routers proxy-репозиториев; каждый монтируется как `/cargo/<имя>`.
/// Имя должно быть допустимым сегментом пути axum (без `{`, `}`, `/`).
pub fn api_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: Arc<Registry<C, B>>,
    cargo: CargoSettings,
    token: StaticToken,
    proxies: TrustedProxies,
    audit: Arc<dyn AuditSink>,
    proxy_repos: Vec<(String, Router)>,
) -> Router {
    let auth = Arc::new(Auth {
        token: Arc::new(token),
        audit,
        dialect: Dialect::Cargo,
    });
    // `layer`, а не `route_layer`: 401 должен закрывать и неизвестные пути, иначе без токена можно
    // перебирать маршруты. Диалект снаружи auth: отказы аутентификации тоже в форме cargo.
    let guarded = |router: Router| {
        router
            .layer(from_fn_with_state(Arc::clone(&auth), require_token))
            .layer(from_fn_with_state(Dialect::Cargo, mark_dialect))
    };
    let mut root = Router::new().nest(
        &format!("{CARGO_PREFIX}/{{repo}}"),
        guarded(hosted::hosted_router(registry, cargo)),
    );
    // Статический сегмент побеждает `{repo}` в маршрутизаторе axum, поэтому proxy-имя
    // не перехватывается hosted-маршрутами (проверено в tests/cargo_proxy.rs).
    for (name, router) in proxy_repos {
        root = root.nest(&format!("{CARGO_PREFIX}/{name}"), guarded(router));
    }
    // Самый внешний слой: контекст запроса нужен и auth, и fallback, и рендеру ошибок.
    root.fallback(not_found)
        .layer(from_fn_with_state(Arc::new(proxies), request_context))
}

async fn cargo_not_found() -> ClientError {
    ClientError::not_found()
}
