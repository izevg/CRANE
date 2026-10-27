//! Два вида ошибочных ответов и единая таблица `RegistryError` → HTTP.

use std::error::Error;

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use crane_app::registry::RegistryError;
use crane_domain::registry::VersionState;
use crane_format_cargo::render_errors;

const INTERNAL_DETAIL: &str = "внутренняя ошибка сервера";

/// Ошибка в форме cargo: `{"errors":[{"detail":"…"}]}`.
#[derive(Debug)]
pub(crate) struct CargoError {
    status: StatusCode,
    detail: String,
}

impl CargoError {
    pub(crate) fn new(status: StatusCode, detail: impl Into<String>) -> Self {
        Self {
            status,
            detail: detail.into(),
        }
    }

    pub(crate) fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, detail)
    }

    pub(crate) fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "не найдено")
    }

    /// 500 с обобщённым текстом: причину видит только журнал.
    pub(crate) fn internal(source: &(dyn Error + 'static)) -> Self {
        let (status, detail) = internal(source);
        Self { status, detail }
    }
}

impl From<RegistryError> for CargoError {
    fn from(error: RegistryError) -> Self {
        let (status, detail) = describe(&error);
        Self { status, detail }
    }
}

impl IntoResponse for CargoError {
    fn into_response(self) -> Response {
        (
            self.status,
            [(header::CONTENT_TYPE, "application/json")],
            render_errors(&self.detail),
        )
            .into_response()
    }
}

/// Ошибка в форме RFC 9457 для не-cargo маршрутов.
#[derive(Debug)]
pub(crate) struct Problem {
    status: StatusCode,
    detail: &'static str,
}

impl Problem {
    pub(crate) const fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            detail: "маршрут не найден",
        }
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        // `detail` и `title` — статические строки без символов, требующих экранирования.
        let body = format!(
            r#"{{"type":"about:blank","title":"{}","status":{},"detail":"{}"}}"#,
            self.status.canonical_reason().unwrap_or("Error"),
            self.status.as_u16(),
            self.detail,
        );
        (
            self.status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            body,
        )
            .into_response()
    }
}

/// Fallback для путей вне cargo-маршрутов.
pub(crate) async fn not_found() -> Problem {
    Problem::not_found()
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

fn internal(source: &(dyn Error + 'static)) -> (StatusCode, String) {
    // Текст источника может содержать адреса и пути хранилища: клиенту он не нужен.
    tracing::error!(error = %chain(source), "внутренняя ошибка запроса");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        INTERNAL_DETAIL.to_owned(),
    )
}

fn describe(error: &RegistryError) -> (StatusCode, String) {
    let status = match error {
        RegistryError::RepositoryNotFound
        | RegistryError::VersionNotFound
        | RegistryError::AssetNotFound => StatusCode::NOT_FOUND,
        RegistryError::NotHosted => StatusCode::METHOD_NOT_ALLOWED,
        RegistryError::VersionExists => StatusCode::CONFLICT,
        RegistryError::Unavailable {
            state: VersionState::Deleted,
        } => StatusCode::GONE,
        RegistryError::Unavailable { .. } => StatusCode::FORBIDDEN,
        RegistryError::DigestMismatch { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        RegistryError::Body(_) => StatusCode::BAD_REQUEST,
        RegistryError::Backend(_) => return internal(error),
    };
    (status, error.to_string())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use std::io;

    use crane_domain::registry::Sha256;

    use super::*;

    #[test]
    fn registry_errors_map_to_distinct_statuses() {
        // Arrange
        let digest = Sha256::from([1; 32]);
        let cases = [
            (RegistryError::RepositoryNotFound, StatusCode::NOT_FOUND),
            (RegistryError::NotHosted, StatusCode::METHOD_NOT_ALLOWED),
            (RegistryError::VersionNotFound, StatusCode::NOT_FOUND),
            (RegistryError::AssetNotFound, StatusCode::NOT_FOUND),
            (RegistryError::VersionExists, StatusCode::CONFLICT),
            (
                RegistryError::Unavailable {
                    state: VersionState::Deleted,
                },
                StatusCode::GONE,
            ),
            (
                RegistryError::Unavailable {
                    state: VersionState::Quarantined,
                },
                StatusCode::FORBIDDEN,
            ),
            (
                RegistryError::Unavailable {
                    state: VersionState::Blocked,
                },
                StatusCode::FORBIDDEN,
            ),
            (
                RegistryError::DigestMismatch {
                    declared: digest,
                    actual: digest,
                },
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                RegistryError::Body(io::Error::other("обрыв")),
                StatusCode::BAD_REQUEST,
            ),
            (
                RegistryError::Backend("сбой".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        // Act
        let actual = cases.map(|(error, expected)| (describe(&error).0, expected));

        // Assert
        for (got, expected) in actual {
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn backend_detail_hides_the_source_message() {
        // Arrange
        let error = RegistryError::Backend("postgres://u:SECRET@h/db".into());

        // Act
        let (_, detail) = describe(&error);

        // Assert
        assert_eq!(detail, INTERNAL_DETAIL);
    }

    #[test]
    fn client_facing_detail_keeps_the_error_text() {
        // Arrange
        let error = RegistryError::VersionExists;

        // Act
        let (_, detail) = describe(&error);

        // Assert
        assert_eq!(detail, "версия уже существует");
    }

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
