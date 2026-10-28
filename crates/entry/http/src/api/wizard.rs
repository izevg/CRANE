//! Публичный router фазы мастера: пока первый запуск не завершён, протоколы отвечают 503.

use axum::Router;
use axum::middleware::from_fn;

use crate::context::render_only;
use crate::error::not_configured;

/// Корневой router публичного слушателя в фазе мастера (20.2). `wizard` - маршруты мастера
/// (SPA, `/api/v1/onboarding*`, `/api/v1/status`); всё остальное, в том числе все пути пакетов,
/// отвечает `503 not-configured` с `Retry-After`.
///
/// Тело ошибки рендерит `render_only`: access-журнал и доверенные прокси маршрутам мастера
/// подключит B4b вместе с лимитом перебора, которому нужен адрес клиента.
pub fn wizard_router(wizard: Router) -> Router {
    wizard.fallback(not_configured).layer(from_fn(render_only))
}
