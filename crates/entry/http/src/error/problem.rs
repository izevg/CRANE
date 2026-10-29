//! Ошибка в форме RFC 9457 для маршрутов вне форматов.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use super::{ErrorBody, NOT_CONFIGURED_DETAIL, NOT_READY_DETAIL, ProblemKind, UNREACHABLE_DETAIL};

#[derive(Debug)]
pub(crate) struct Problem {
    status: StatusCode,
    detail: &'static str,
    kind: ProblemKind,
}

impl Problem {
    pub(crate) const fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            detail: "маршрут не найден",
            kind: ProblemKind::Blank,
        }
    }
}

impl Problem {
    /// Проба готовности: хранилище не отвечает (тот же тип и текст, что у `ClientError::unreachable`).
    pub(crate) const fn unreachable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            detail: UNREACHABLE_DETAIL,
            kind: ProblemKind::StorageUnreachable,
        }
    }

    /// Проба готовности: хранилище ответило ошибкой (конфигурация или баг), а не молчанием.
    /// Сервис не готов, но «повторите позже» не обещаем: тип общий.
    pub(crate) const fn not_ready() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            detail: NOT_READY_DETAIL,
            kind: ProblemKind::Blank,
        }
    }
}

impl Problem {
    /// Фаза мастера: сервер жив, но не настроен. `Retry-After` ставит общий рендер ответов 503.
    pub(crate) const fn not_configured() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            detail: NOT_CONFIGURED_DETAIL,
            kind: ProblemKind::NotConfigured,
        }
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let mut response = self.status.into_response();
        response.extensions_mut().insert(ErrorBody::Problem {
            title: self.status.canonical_reason().unwrap_or("Error"),
            detail: self.detail,
            kind: self.kind,
        });
        response
    }
}

/// Fallback фазы мастера: на любой путь и метод.
pub(crate) async fn not_configured() -> Problem {
    Problem::not_configured()
}

/// Fallback для путей вне cargo-маршрутов.
pub(crate) async fn not_found() -> Problem {
    Problem::not_found()
}
