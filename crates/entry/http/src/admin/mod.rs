//! Административный listener: пробы живости и готовности.

use axum::Router;
use axum::middleware::from_fn;
use axum::routing::get;

use crate::context::render_only;
use crate::error::not_found;

/// Пробы не требуют токена: их дёргает оркестратор, а не клиент реестра.
///
/// Готовность пока не проверяет хранилище (оно в памяти), см. DEBT.md.
pub fn admin_router() -> Router {
    Router::new()
        .route("/healthz/live", get(ok))
        .route("/healthz/ready", get(ok))
        .fallback(not_found)
        // Только рендер тела ошибки: access-журнал и доверенные прокси пробам не нужны.
        .layer(from_fn(render_only))
}

async fn ok() -> &'static str {
    "ok"
}
