//! Ошибка в форме RFC 9457 для маршрутов вне форматов.

use std::borrow::Cow;
use std::error::Error;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value};

use super::{ErrorBody, INTERNAL_DETAIL, NOT_READY_DETAIL, ProblemKind, UNREACHABLE_DETAIL, chain};

#[derive(Debug)]
pub(crate) struct Problem {
    status: StatusCode,
    detail: Cow<'static, str>,
    kind: ProblemKind,
    extensions: Map<String, Value>,
}

impl Problem {
    pub(crate) fn new(
        status: StatusCode,
        detail: impl Into<Cow<'static, str>>,
        kind: ProblemKind,
    ) -> Self {
        Self {
            status,
            detail: detail.into(),
            kind,
            extensions: Map::new(),
        }
    }

    /// Добавляет член тела сверх стандартных (`errors`, `var`): стандартные он не затирает.
    #[must_use]
    pub(crate) fn with(mut self, name: &str, value: Value) -> Self {
        self.extensions.insert(name.to_owned(), value);
        self
    }

    pub(crate) fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "маршрут не найден",
            ProblemKind::Blank,
        )
    }

    /// Проба готовности: хранилище не отвечает (тот же тип и текст, что у `ClientError::unreachable`).
    pub(crate) fn unreachable() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            UNREACHABLE_DETAIL,
            ProblemKind::StorageUnreachable,
        )
    }

    /// Проба готовности: хранилище ответило ошибкой (конфигурация или баг), а не молчанием.
    /// Сервис не готов, но «повторите позже» не обещаем: тип общий.
    pub(crate) fn not_ready() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            NOT_READY_DETAIL,
            ProblemKind::Blank,
        )
    }

    /// 500 с обобщённым текстом: источник (адреса, пути) остаётся в журнале.
    pub(crate) fn internal(source: &(dyn Error + 'static)) -> Self {
        tracing::error!(error = %chain(source), "внутренняя ошибка запроса");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            INTERNAL_DETAIL,
            ProblemKind::Blank,
        )
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let mut response = self.status.into_response();
        response.extensions_mut().insert(ErrorBody::Problem {
            title: self.status.canonical_reason().unwrap_or("Error"),
            detail: self.detail,
            kind: self.kind,
            extensions: self.extensions,
        });
        response
    }
}

/// Fallback для путей вне cargo-маршрутов.
pub(crate) async fn not_found() -> Problem {
    Problem::not_found()
}
