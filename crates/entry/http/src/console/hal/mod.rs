//! Ответы консоли в `application/hal+json`: `ETag`, условные запросы и кэш-заголовки.

use std::fmt::Write as _;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use crane_domain::settings::Version;
use serde_json::Value;
use sha2::{Digest, Sha256};

const SETTINGS_PATH: &str = "/api/v1/settings";
const NO_STORE: HeaderValue = HeaderValue::from_static("no-store");
const PRIVATE: HeaderValue = HeaderValue::from_static("private");
/// Тело зависит от токена (`/status` без него короче) или от сессии (v0.4.0): общий кэш не должен
/// отдавать одно вместо другого.
const VARY_CREDENTIALS: HeaderValue = HeaderValue::from_static("Authorization, Cookie");
const HAL_JSON: HeaderValue = HeaderValue::from_static("application/hal+json");
/// Длина дайджеста в `ETag`: хватает, чтобы отличить представления, и не раздувает заголовок.
const TAG_DIGEST_BYTES: usize = 16;

/// Ответ с телом `body`. `ETag` — версия строки настройки, если она есть.
pub(crate) fn json(status: StatusCode, body: &Value, version: Option<Version>) -> Response {
    let mut response = respond(status, body);
    if let Some(version) = version
        && let Ok(tag) = HeaderValue::from_str(&format!("\"{}\"", version.get()))
    {
        response.headers_mut().insert(header::ETAG, tag);
    }
    response
}

/// Ответ с `ETag` от тела; совпавший `If-None-Match` даёт 304 без тела.
pub(crate) fn cacheable(request: &HeaderMap, body: &Value) -> Response {
    let tag = tag_of(body);
    let mut response = if matches(request, &tag) {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        response
    } else {
        respond(StatusCode::OK, body)
    };
    let headers = response.headers_mut();
    if let Ok(tag) = HeaderValue::from_str(&tag) {
        headers.insert(header::ETAG, tag);
    }
    response
}

/// Единая политика кэша консоли, по пути ответа (включая 401 и 404): настройки не хранятся нигде,
/// профиль и статус — только в приватном кэше и с ключом по учётным данным. Остальным ответам
/// заголовки не ставятся: `ETag` и 304 у `/me` и `/status` остаются.
pub(crate) async fn cache_policy(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    if path == SETTINGS_PATH || path.starts_with("/api/v1/settings/") {
        headers.insert(header::CACHE_CONTROL, NO_STORE);
    } else if path == "/api/v1/me" || path == "/api/v1/status" {
        headers.insert(header::CACHE_CONTROL, PRIVATE);
        headers.insert(header::VARY, VARY_CREDENTIALS);
    }
    response
}

fn respond(status: StatusCode, body: &Value) -> Response {
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HAL_JSON);
    response
}

/// Сильный валидатор от канонического текста тела.
fn tag_of(body: &Value) -> String {
    let digest = Sha256::digest(body.to_string().as_bytes());
    let hex = digest
        .iter()
        .take(TAG_DIGEST_BYTES)
        .fold(String::new(), |mut out, byte| {
            // Запись в `String` не отказывает.
            let _ = write!(out, "{byte:02x}");
            out
        });
    format!("\"{hex}\"")
}

/// `If-None-Match` по слабому сравнению (RFC 9110, 13.1.2): `*`, список тегов, префикс `W/`.
fn matches(request: &HeaderMap, tag: &str) -> bool {
    request
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|candidate| {
            candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == tag
        })
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
