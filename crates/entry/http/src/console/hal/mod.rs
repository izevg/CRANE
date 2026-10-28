//! Ответы консоли в `application/hal+json`: `ETag`, условные запросы и кэш-заголовки.

use std::fmt::Write as _;

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use crane_domain::settings::Version;
use serde_json::Value;
use sha2::{Digest, Sha256};

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

/// Ответ с `ETag` от тела и `Cache-Control`; совпавший `If-None-Match` даёт 304 без тела.
pub(crate) fn cacheable(request: &HeaderMap, body: &Value, cache: &'static str) -> Response {
    let tag = tag_of(body);
    let mut response = if matches(request, &tag) {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        response
    } else {
        respond(StatusCode::OK, body)
    };
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    if let Ok(tag) = HeaderValue::from_str(&tag) {
        headers.insert(header::ETAG, tag);
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
