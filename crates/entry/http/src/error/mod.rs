//! Ошибочные ответы и единая таблица `RegistryError` → HTTP. Форма тела — свойство точки монтирования (`Format`).

use std::borrow::Cow;
use std::error::Error;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use crane_domain::registry::Format;
use serde_json::{Map, Value};

mod kind;
mod problem;
mod status;

pub(crate) use kind::ProblemKind;
pub(crate) use problem::{Problem, not_found};

pub(crate) const INTERNAL_DETAIL: &str = "внутренняя ошибка сервера";
/// Единственный текст клиенту при недоступном хранилище: адреса и причины остаются в журнале.
const UNREACHABLE_DETAIL: &str = "хранилище временно недоступно";
const NOT_READY_DETAIL: &str = "хранилище не готово";
const UPSTREAM_DETAIL: &str = "upstream недоступен";
const NOT_FOUND_DETAIL: &str = "не найдено";

/// Что именно писать в тело ошибки. Ответ несёт его в расширениях с пустым телом:
/// id запроса известен только внешнему слою, он и рендерит тело (`context::render`).
#[derive(Debug, Clone)]
pub(crate) enum ErrorBody {
    /// Текст для клиента; оболочку (`{"errors":[…]}`, `{"error":…}` или RFC 9457) выбирает формат (`formats::error_body`).
    Detail { detail: String, kind: ProblemKind },
    /// RFC 9457; `extensions` — дополнительные члены (`errors`, `var`), стандартные они не затирают.
    Problem {
        title: &'static str,
        detail: Cow<'static, str>,
        kind: ProblemKind,
        extensions: Map<String, Value>,
    },
}

/// Ошибка для клиента формата: статус и текст, без привязки к проводной форме.
#[derive(Debug)]
pub(crate) struct ClientError {
    status: StatusCode,
    detail: String,
    kind: ProblemKind,
}

impl ClientError {
    pub(crate) fn new(status: StatusCode, detail: impl Into<String>) -> Self {
        Self {
            status,
            detail: detail.into(),
            kind: ProblemKind::Blank,
        }
    }

    pub(crate) fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, detail)
    }

    pub(crate) fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, NOT_FOUND_DETAIL)
    }

    /// 500 с обобщённым текстом: причину видит только журнал.
    pub(crate) fn internal(source: &(dyn Error + 'static)) -> Self {
        // Текст источника может содержать адреса и пути хранилища: клиенту он не нужен.
        tracing::error!(error = %chain(source), "внутренняя ошибка запроса");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_DETAIL)
    }

    /// 503: хранилище не отвечает, повтор позже имеет смысл. Это отказ среды, а не баг, поэтому warn.
    pub(crate) fn unreachable(source: &(dyn Error + 'static)) -> Self {
        // Цепочка может содержать хост хранилища: она только в журнале.
        tracing::warn!(error = %chain(source), "хранилище недоступно");
        Self {
            kind: ProblemKind::StorageUnreachable,
            ..Self::new(StatusCode::SERVICE_UNAVAILABLE, UNREACHABLE_DETAIL)
        }
    }
}

impl IntoResponse for ClientError {
    fn into_response(self) -> Response {
        let mut response = self.status.into_response();
        response.extensions_mut().insert(ErrorBody::Detail {
            detail: self.detail,
            kind: self.kind,
        });
        response
    }
}

/// Слой router'а формата (`from_fn_with_state(format, mark_format)`): снаружи auth, чтобы 401 и 403
/// тоже получили форму своего формата. Ошибка не знает, как её покажут: тело рендерит внешний слой
/// по формату из расширений ответа.
pub(crate) async fn mark_format(
    State(format): State<Format>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    response.extensions_mut().insert(format);
    response
}

/// Цепочка причин одной строкой: «ошибка: причина: первопричина».
pub(crate) fn chain(error: &(dyn Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

#[cfg(test)]
mod tests {
    use std::io;

    use crane_app::registry::RegistryError;

    use super::*;

    #[test]
    fn chain_joins_causes_with_colon() {
        // Arrange
        let error = RegistryError::Body(io::Error::other("обрыв"));

        // Act
        let text = chain(&error);

        // Assert
        assert_eq!(text, "ошибка чтения тела запроса: обрыв");
    }
}
