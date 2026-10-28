//! `GET /me` (профиль и навигация) и `GET /status` (версия, режим, фаза; `restart_required` только под токеном).

use std::sync::Arc;

use axum::Router;
use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::get;
use crane_app::settings::Settings;
use crane_domain::access::{Caller, PrincipalKind};
use crane_ports::settings::SettingsStore;
use serde_json::{Map, Value, json};

use super::NavLink;
use super::hal;
use crate::error::Problem;

pub(super) const STATUS_HREF: &str = "/api/v1/status";
const ME_HREF: &str = "/api/v1/me";
/// Бейдж баннера и профиль меняются редко; UI опрашивает статус с `ETag`.
const CACHE: &str = "private, max-age=30";
/// До v0.4.0 единственная роль статического токена; ролевая модель — в 14.12.
const ROLE_ADMIN: &str = "platform-admin";
/// Режим `SoD` до v0.4.0 фиксирован: конфликтов быть не может, у токена одна роль.
const SOD_MODE: &str = "warn";
/// Рабочая фаза процесса; фазу мастера отдаёт первая фаза `serve` (B4b).
const PHASE_SERVING: &str = "serving";
const MODE_NORMAL: &str = "normal";

pub(super) fn routes<S: SettingsStore + 'static>(
    nav: Vec<NavLink>,
    settings: Arc<Settings<S>>,
) -> Router {
    let me = Router::new()
        .route("/me", get(me))
        .with_state(Arc::<[NavLink]>::from(nav));
    let status = Router::new()
        .route("/status", get(status::<S>))
        .with_state(settings);
    me.merge(status)
}

async fn me(
    State(nav): State<Arc<[NavLink]>>,
    Extension(caller): Extension<Caller>,
    headers: HeaderMap,
) -> Response {
    let mut links = Map::new();
    links.insert("self".to_owned(), json!({"href": ME_HREF}));
    for link in nav.iter() {
        links.insert(link.rel.to_owned(), json!({"href": link.href}));
    }
    let id = &caller.principal.id;
    let body = json!({
        "principal": {"id": id, "kind": kind_name(caller.principal.kind), "display": id},
        "roles": [ROLE_ADMIN],
        "workspaces": [],
        "sod": {"mode": SOD_MODE, "conflict": false},
        "_links": links,
    });
    hal::cacheable(&headers, &body, CACHE)
}

async fn status<S: SettingsStore + 'static>(
    State(settings): State<Arc<Settings<S>>>,
    caller: Option<Extension<Caller>>,
    headers: HeaderMap,
) -> Result<Response, Problem> {
    let mut body = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "mode": MODE_NORMAL,
        "phase": PHASE_SERVING,
        "_links": {"self": {"href": STATUS_HREF}},
    });
    // Публичная часть не зависит от хранилища настроек; закрытая спрашивает его и честно падает с ним.
    if caller.is_some() {
        let restart_required = settings.view().await?.restart_required;
        body["restart_required"] = Value::Bool(restart_required);
    }
    Ok(hal::cacheable(&headers, &body, CACHE))
}

const fn kind_name(kind: PrincipalKind) -> &'static str {
    match kind {
        PrincipalKind::Human => "human",
        PrincipalKind::Robot => "robot",
        PrincipalKind::Workload => "workload",
        PrincipalKind::Agent => "agent",
    }
}
