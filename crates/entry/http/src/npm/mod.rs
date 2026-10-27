//! Маршруты npm-реестра под `/npm/{repo}`: hosted и proxy-репозитории.
//!
//! Клиенту npm нужны пути вида `/{pkg}/-/{file}` и `@scope%2fname`, которые обычными шаблонами axum
//! не выразить, поэтому на точку монтирования приходится один обработчик: он зовёт `Route::parse` формата.

mod hosted;
mod proxy;

use axum::http::{Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use crane_domain::access::Caller;
use crane_format_npm::{Route, RouteError, render_whoami};

pub(crate) use hosted::hosted_router;
pub use proxy::npm_proxy_router;

use crate::error::{ClientError, chain};

/// Точка монтирования npm-маршрутов; от неё же строится база ссылок на tarball.
pub(crate) const NPM_PREFIX: &str = "/npm";
const JSON: &str = "application/json";
const EMPTY_OBJECT: &str = "{}";
const AUDIT_UNSUPPORTED: &str = "аудит уязвимостей через реестр не поддерживается";

/// Настройки npm-маршрутов.
#[derive(Debug, Clone)]
pub struct NpmSettings {
    /// Внешний адрес без завершающего '/': от него строятся ссылки на tarball в packument.
    pub public_url: String,
    /// Предел тела публикации. Вложение приходит в base64 внутри JSON, то есть на треть больше
    /// самого tarball: ограничение применяется к документу, а не к архиву.
    pub max_publish_bytes: usize,
}

impl NpmSettings {
    /// База ссылок на tarball: точка монтирования, с которой пришёл запрос.
    pub(crate) fn mount(&self, repo: &str) -> String {
        format!("{}{NPM_PREFIX}/{repo}", self.public_url)
    }
}

pub(crate) async fn npm_not_found() -> ClientError {
    ClientError::not_found()
}

pub(crate) fn json_ok(body: String) -> Response {
    ([(header::CONTENT_TYPE, JSON)], body).into_response()
}

/// Разбор маршрута. HEAD читает то же, что GET (тело отбрасывает сервер).
pub(crate) fn parse_route(method: &Method, path: &str) -> Result<Route, ClientError> {
    let verb = if method == Method::HEAD {
        "GET"
    } else {
        method.as_str()
    };
    Route::parse(verb, path).map_err(|error| match error {
        RouteError::NotFound => ClientError::not_found(),
        RouteError::Name(name) => ClientError::bad_request(chain(&name)),
    })
}

/// Маршруты, не зависящие от репозитория: одинаковы для hosted и proxy.
pub(crate) fn service_reply(route: &Route, caller: &Caller) -> Option<Response> {
    match route {
        Route::Whoami => Some(json_ok(render_whoami(&caller.principal.id))),
        Route::Ping => Some(json_ok(EMPTY_OBJECT.to_owned())),
        // Список зависимостей проекта нельзя отдавать наружу, поэтому аудит не проксируется никуда.
        Route::Audit => {
            Some(ClientError::new(StatusCode::NOT_IMPLEMENTED, AUDIT_UNSUPPORTED).into_response())
        }
        _ => None,
    }
}
