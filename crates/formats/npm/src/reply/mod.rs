//! Тела ответов в диалекте npm.

use serde::Serialize;

/// Сериализация строковых структур не отказывает; пустое тело на недостижимый отказ лучше паники.
pub(crate) fn to_json(value: &impl Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// `{"error": "..."}`.
#[must_use]
pub fn render_error(detail: &str) -> String {
    #[derive(Serialize)]
    struct Body<'a> {
        error: &'a str,
    }
    to_json(&Body { error: detail })
}

/// `{"ok":true}`.
#[must_use]
pub fn render_ok() -> String {
    #[derive(Serialize)]
    struct Body {
        ok: bool,
    }
    to_json(&Body { ok: true })
}

/// Ответ `GET /-/whoami`.
#[must_use]
pub fn render_whoami(username: &str) -> String {
    #[derive(Serialize)]
    struct Body<'a> {
        username: &'a str,
    }
    to_json(&Body { username })
}
