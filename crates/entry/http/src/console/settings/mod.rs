//! `GET/PUT/DELETE /settings` поверх `crane_app::settings::Settings`.

mod dto;
mod precondition;

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::Response;
use axum::routing::get;
use crane_app::settings::{SettingView, Settings};
use crane_domain::access::Caller;
use crane_ports::settings::SettingsStore;
use serde_json::{Value, json};

use super::hal;
use crate::error::{Problem, ProblemKind};

pub(super) const SETTINGS_HREF: &str = "/api/v1/settings";
/// Предел длины ключа из схемы `SettingKey`.
const KEY_MAX_LEN: usize = 96;
const EMBEDDED: &str = "crane:settings";
const ORIGINS: [&str; 3] = ["env", "stored", "default"];

/// Use cases и то, чего в них не видно: пишет ли хранилище (`Settings` отдаёт только `view`).
pub(super) struct Api<S> {
    settings: Arc<Settings<S>>,
    persistent: bool,
}

pub(super) fn routes<S: SettingsStore + 'static>(
    settings: Arc<Settings<S>>,
    persistent: bool,
) -> Router {
    Router::new()
        .route("/settings", get(list::<S>))
        .route(
            "/settings/{key}",
            get(read::<S>).put(write::<S>).delete(remove::<S>),
        )
        .with_state(Arc::new(Api {
            settings,
            persistent,
        }))
}

async fn list<S: SettingsStore + 'static>(
    State(api): State<Arc<Api<S>>>,
    uri: Uri,
) -> Result<Response, Problem> {
    let filter = Filter::parse(uri.query())?;
    let list = api.settings.view().await?;
    let items: Vec<Value> = list
        .settings
        .iter()
        .filter(|view| filter.accepts(view))
        .map(dto::setting)
        .collect();
    let body = json!({
        "_embedded": {EMBEDDED: items},
        "restart_required": list.restart_required,
        "persistent": api.persistent,
        "_links": {"self": {"href": SETTINGS_HREF}},
    });
    Ok(hal::json(StatusCode::OK, &body, None))
}

async fn read<S: SettingsStore + 'static>(
    State(api): State<Arc<Api<S>>>,
    Path(key): Path<String>,
) -> Result<Response, Problem> {
    ensure_key(&key)?;
    current(&api, &key).await
}

async fn write<S: SettingsStore + 'static>(
    State(api): State<Arc<Api<S>>>,
    Extension(caller): Extension<Caller>,
    Path(key): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Problem> {
    ensure_key(&key)?;
    let expected = precondition::for_put(&headers)?;
    let raw = value_of(&body)?;
    api.settings.put(&caller, &key, raw, expected).await?;
    current(&api, &key).await
}

async fn remove<S: SettingsStore + 'static>(
    State(api): State<Arc<Api<S>>>,
    Extension(caller): Extension<Caller>,
    Path(key): Path<String>,
    headers: HeaderMap,
) -> Result<Response, Problem> {
    ensure_key(&key)?;
    let expected = precondition::for_delete(&headers)?;
    api.settings.delete(&caller, &key, expected).await?;
    current(&api, &key).await
}

/// Настройка в её нынешнем состоянии; `ETag` — версия строки, если она есть.
async fn current<S: SettingsStore + 'static>(api: &Api<S>, key: &str) -> Result<Response, Problem> {
    let list = api.settings.view().await?;
    let view = list
        .settings
        .iter()
        .find(|view| view.def.key == key)
        .ok_or_else(unknown)?;
    let version = view.stored.as_ref().map(|row| row.version);
    Ok(hal::json(StatusCode::OK, &dto::setting(view), version))
}

/// Ключ вне шаблона `^[a-z][a-z0-9_]*$` не доходит до use case, а значит и до журнала аудита.
fn ensure_key(key: &str) -> Result<(), Problem> {
    let mut chars = key.chars();
    let well_formed = key.len() <= KEY_MAX_LEN
        && chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if well_formed { Ok(()) } else { Err(unknown()) }
}

fn unknown() -> Problem {
    Problem::new(
        StatusCode::NOT_FOUND,
        "настройка не найдена",
        ProblemKind::Blank,
    )
}

/// Тело `{"value": "<строка>"}`. Содержимое в сообщение об ошибке не попадает: там может быть секрет.
fn value_of(body: &[u8]) -> Result<String, Problem> {
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(mut members)) => match members.remove("value") {
            Some(Value::String(value)) => Ok(value),
            _ => Err(malformed()),
        },
        _ => Err(malformed()),
    }
}

fn malformed() -> Problem {
    Problem::new(
        StatusCode::BAD_REQUEST,
        "ожидается JSON {\"value\": \"<строка>\"}",
        ProblemKind::Blank,
    )
}

/// Параметры `group` и `origin`. Значения — имена из закрытых наборов, декодирование не нужно.
struct Filter<'a> {
    group: Option<&'a str>,
    origin: Option<&'a str>,
}

impl<'a> Filter<'a> {
    fn parse(query: Option<&'a str>) -> Result<Self, Problem> {
        let mut filter = Self {
            group: None,
            origin: None,
        };
        for pair in query.unwrap_or_default().split('&') {
            match pair.split_once('=') {
                Some(("group", group)) => filter.group = filter.group.or(Some(group)),
                Some(("origin", origin)) => filter.origin = filter.origin.or(Some(origin)),
                _ => {}
            }
        }
        if filter
            .origin
            .is_some_and(|origin| !ORIGINS.contains(&origin))
        {
            return Err(Problem::new(
                StatusCode::BAD_REQUEST,
                "origin: ожидается env, stored или default",
                ProblemKind::Blank,
            ));
        }
        Ok(filter)
    }

    fn accepts(&self, view: &SettingView) -> bool {
        self.group.is_none_or(|group| view.def.group == group)
            && self
                .origin
                .is_none_or(|origin| dto::origin_name(view.origin()) == origin)
    }
}
