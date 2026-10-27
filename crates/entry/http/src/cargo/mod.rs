//! Маршруты cargo sparse-реестра под `/cargo/{repo}`: hosted и proxy-репозитории.

mod auth;
mod hosted;

use std::sync::Arc;

use axum::Router;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::{ETag, IfNoneMatch};
use bytes::Bytes;
use crane_app::registry::Registry;
use crane_domain::registry::Sha256 as Digest;
use crane_ports::audit::AuditSink;
use crane_ports::registry::{BlobStore, Catalog};
use sha2::{Digest as _, Sha256};

use auth::Auth;
pub use auth::StaticToken;

use crate::context::{TrustedProxies, request_context};
use crate::error::{CargoError, not_found};

/// Точка монтирования cargo-маршрутов; от неё же строится база в `config.json`.
const CARGO_PREFIX: &str = "/cargo";
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
pub fn api_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: Arc<Registry<C, B>>,
    cargo: CargoSettings,
    token: StaticToken,
    proxies: TrustedProxies,
    audit: Arc<dyn AuditSink>,
) -> Router {
    let auth = Arc::new(Auth { token, audit });
    // `layer`, а не `route_layer`: 401 должен закрывать и неизвестные пути,
    // иначе без токена можно перебирать маршруты.
    let guarded =
        |router: Router| router.layer(from_fn_with_state(Arc::clone(&auth), auth::require_token));
    let root = Router::new().nest(
        &format!("{CARGO_PREFIX}/{{repo}}"),
        guarded(hosted::hosted_router(registry, cargo)),
    );
    // Самый внешний слой: контекст запроса нужен и auth, и fallback, и рендеру ошибок.
    root.fallback(not_found)
        .layer(from_fn_with_state(Arc::new(proxies), request_context))
}

async fn cargo_not_found() -> CargoError {
    CargoError::not_found()
}

/// Ответ с телом индекса: общее правило для hosted и proxy. `ETag` считается по тем самым
/// байтам, что уходят клиенту, поэтому условный запрос работает и для проксированного индекса.
fn index_response(
    body: Bytes,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Result<Response, CargoError> {
    let etag = etag_of(&body)?;
    let unchanged =
        if_none_match.is_some_and(|TypedHeader(condition)| !condition.precondition_passes(&etag));
    if unchanged {
        return Ok((StatusCode::NOT_MODIFIED, TypedHeader(etag)).into_response());
    }
    Ok((
        TypedHeader(etag),
        [(header::CONTENT_TYPE, HeaderValue::from_static(INDEX_TYPE))],
        body,
    )
        .into_response())
}

fn etag_of(body: &[u8]) -> Result<ETag, CargoError> {
    let digest = Digest::from(<[u8; 32]>::from(Sha256::digest(body)));
    format!("\"{digest}\"")
        .parse()
        .map_err(|error| CargoError::internal(&error))
}
