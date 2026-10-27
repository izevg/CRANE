//! Административный listener: пробы живости и готовности.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::middleware::from_fn;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use crane_app::registry::Registry;
use crane_ports::registry::{BlobStore, Catalog};
use tokio::time::timeout;

use crate::context::render_only;
use crate::error::{Problem, chain, not_found};

/// Клиент пробы ждёт 2 с, healthcheck compose — 3 с: ответ «не готов» должен прийти раньше,
/// иначе причину увидит только оркестратор по таймауту.
const READY_BUDGET: Duration = Duration::from_millis(1500);

/// Пробы не требуют токена: их дёргает оркестратор, а не клиент реестра.
///
/// Живость хранилищ не трогает: недоступная БД не повод убивать процесс.
/// Готовность спрашивает оба хранилища через `Registry::ready`.
pub fn admin_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: Arc<Registry<C, B>>,
) -> Router {
    Router::new()
        .route("/healthz/live", get(live))
        .route("/healthz/ready", get(ready::<C, B>))
        .fallback(not_found)
        .with_state(registry)
        // Только рендер тела ошибки: access-журнал и доверенные прокси пробам не нужны.
        .layer(from_fn(render_only))
}

async fn live() -> &'static str {
    "ok"
}

async fn ready<C: Catalog + 'static, B: BlobStore + 'static>(
    State(registry): State<Arc<Registry<C, B>>>,
) -> Response {
    match timeout(READY_BUDGET, registry.ready()).await {
        Ok(Ok(())) => "ok".into_response(),
        Ok(Err(error)) => {
            tracing::warn!(error = %chain(&error), "хранилище недоступно");
            Problem::storage_unavailable().into_response()
        }
        Err(_) => {
            tracing::warn!("проба хранилищ не уложилась в {READY_BUDGET:?}");
            Problem::storage_unavailable().into_response()
        }
    }
}
