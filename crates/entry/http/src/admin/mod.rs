//! Административный listener: пробы живости и готовности.

use axum::Router;
use axum::routing::get;

use crate::error::not_found;

/// Пробы не требуют токена: их дёргает оркестратор, а не клиент реестра.
///
/// Готовность пока не проверяет хранилище (оно в памяти), см. DEBT.md.
pub fn admin_router() -> Router {
    Router::new()
        .route("/healthz/live", get(ok))
        .route("/healthz/ready", get(ok))
        .fallback(not_found)
}

async fn ok() -> &'static str {
    "ok"
}
