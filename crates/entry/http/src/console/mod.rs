//! Консоль Admin API на публичном слушателе: `/api/v1/*` под `Authorization: Bearer <CRANE_TOKEN>`.
//!
//! Разделы других потоков (`SecOps` и далее) монтируются как [`Section`]: роутер с уже применённым
//! состоянием и список ссылок, которые `GET /me` покажет только если раздел смонтирован.

mod access;
mod hal;
mod problem;
mod profile;
mod settings;

use std::sync::Arc;

use axum::Router;
use axum::middleware::from_fn_with_state;
use crane_app::settings::Settings;
use crane_ports::audit::AuditSink;
use crane_ports::settings::SettingsStore;

use crate::auth::StaticToken;
use crate::context::{TrustedProxies, request_context};
use crate::error::not_found;

use self::access::{Access, require_bearer};

/// Версия API в пути; `href` разделов и ссылки HAL абсолютны от корня сервера.
const PREFIX: &str = "/api/v1";

/// Ссылка `GET /me` на раздел: `rel` из словаря HAL (`crane:findings`), `href` от корня сервера.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavLink {
    pub rel: &'static str,
    pub href: &'static str,
}

/// Раздел консоли. Пути роутера заданы относительно `/api/v1`, состояние уже применено
/// (`router.with_state(state)`), поэтому разделы с разным состоянием лежат в одном списке.
/// Роутер раздела не задаёт `fallback`: он общий для всей консоли.
#[derive(Debug)]
pub struct Section {
    pub router: Router,
    /// Ссылки, которые раздел даёт в `/me`; пусто — раздел в навигации не виден.
    pub nav: &'static [NavLink],
}

/// Разделы самой консоли, они видны в `/me` всегда.
const OWN_NAV: [NavLink; 2] = [
    NavLink {
        rel: "crane:status",
        href: profile::STATUS_HREF,
    },
    NavLink {
        rel: "crane:settings",
        href: settings::SETTINGS_HREF,
    },
];

/// Консоль: `/me`, `/status`, `/settings` и смонтированные `sections` под `/api/v1`.
///
/// `persistent` — пишет ли хранилище настроек (`false` в режиме memory); `token` и `audit` — те же,
/// что у `api_router`: отказы входа пишутся в журнал одним и тем же событием. Результат целиком
/// самодостаточен (свой слой контекста запроса), его сливают с `api_router` через `Router::merge`.
pub fn console_router<S: SettingsStore + 'static>(
    settings: Arc<Settings<S>>,
    persistent: bool,
    sections: Vec<Section>,
    token: StaticToken,
    audit: Arc<dyn AuditSink>,
    proxies: TrustedProxies,
) -> Router {
    let nav: Vec<NavLink> = OWN_NAV
        .iter()
        .chain(sections.iter().flat_map(|section| section.nav))
        .copied()
        .collect();
    let mut v1 = Router::new()
        .merge(profile::routes(nav, Arc::clone(&settings)))
        .merge(settings::routes(settings, persistent));
    for section in sections {
        v1 = v1.merge(section.router);
    }
    // `layer`, а не `route_layer`: 401 закрывает и неизвестные пути, иначе без токена
    // можно перебирать маршруты консоли.
    Router::new()
        .nest(PREFIX, v1.fallback(not_found))
        .layer(from_fn_with_state(
            Arc::new(Access { token, audit }),
            require_bearer,
        ))
        .layer(from_fn_with_state(Arc::new(proxies), request_context))
}
