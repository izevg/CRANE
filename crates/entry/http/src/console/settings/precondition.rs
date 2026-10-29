//! Условия записи: `If-None-Match: *` создаёт строку, `If-Match: "<версия>"` обновляет или удаляет.

use axum::http::{HeaderMap, StatusCode, header};
use crane_domain::settings::Version;

use crate::error::{Problem, ProblemKind};

const CREATE_MARK: &str = "*";

/// PUT: `None` — создание, `Some` — обновление этой версии.
pub(super) fn for_put(headers: &HeaderMap) -> Result<Option<Version>, Problem> {
    let if_match = headers.get_all(header::IF_MATCH);
    let if_none_match = headers.get_all(header::IF_NONE_MATCH);
    match (if_match.iter().count(), if_none_match.iter().count()) {
        (0, 0) => Err(required()),
        (1, 0) => version(headers).map(Some),
        (0, 1) => {
            if single(headers, header::IF_NONE_MATCH) == Some(CREATE_MARK) {
                Ok(None)
            } else {
                Err(bad("If-None-Match допускает только *"))
            }
        }
        _ => Err(bad("укажите либо If-Match, либо If-None-Match: *")),
    }
}

/// DELETE: версия обязательна.
pub(super) fn for_delete(headers: &HeaderMap) -> Result<Version, Problem> {
    if headers.contains_key(header::IF_MATCH) {
        version(headers)
    } else {
        Err(required())
    }
}

/// `"<число>"` ровно в одном заголовке: слабые теги и списки не принимаются.
fn version(headers: &HeaderMap) -> Result<Version, Problem> {
    single(headers, header::IF_MATCH)
        .and_then(|value| value.strip_prefix('"')?.strip_suffix('"'))
        .and_then(|digits| digits.parse().ok())
        .and_then(Version::new)
        .ok_or_else(|| bad("If-Match ожидает версию в кавычках, например \"3\""))
}

fn single(headers: &HeaderMap, name: header::HeaderName) -> Option<&str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    value.to_str().ok().map(str::trim)
}

fn required() -> Problem {
    Problem::new(
        StatusCode::PRECONDITION_REQUIRED,
        "укажите If-None-Match: * для создания или If-Match с версией для изменения",
        ProblemKind::PreconditionRequired,
    )
}

fn bad(detail: &'static str) -> Problem {
    Problem::new(StatusCode::BAD_REQUEST, detail, ProblemKind::Blank)
}

#[cfg(test)]
#[path = "tests/precondition.rs"]
mod tests;
