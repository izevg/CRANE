//! Ошибочные ответы и единая таблица `RegistryError` → HTTP. Форма тела — свойство точки монтирования (`Dialect`).

use std::error::Error;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use crane_app::proxy::ProxyError;
use crane_app::registry::RegistryError;
use crane_domain::registry::VersionState;

const INTERNAL_DETAIL: &str = "внутренняя ошибка сервера";
/// Единственный текст клиенту при недоступном хранилище: адреса и причины остаются в журнале.
const UNREACHABLE_DETAIL: &str = "хранилище временно недоступно";
const UPSTREAM_DETAIL: &str = "upstream недоступен";

/// Что именно писать в тело ошибки. Ответ несёт его в расширениях с пустым телом:
/// id запроса известен только внешнему слою, он и рендерит тело (`context::render`).
#[derive(Debug, Clone)]
pub(crate) enum ErrorBody {
    /// Текст для клиента; оболочку (`{"errors":[…]}`, `{"error":…}` или RFC 9457) выбирает `Dialect`.
    Detail(String),
    /// RFC 9457.
    Problem {
        title: &'static str,
        detail: &'static str,
    },
}

/// Ошибка для клиента формата: статус и текст, без привязки к проводной форме.
#[derive(Debug)]
pub(crate) struct ClientError {
    status: StatusCode,
    detail: String,
}

impl ClientError {
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

impl From<RegistryError> for ClientError {
    fn from(error: RegistryError) -> Self {
        let (status, detail) = describe(&error);
        Self { status, detail }
    }
}

/// Клиенту proxy сообщает только «upstream недоступен»: адреса, причины отказа и суммы остаются в журнале.
impl From<ProxyError> for ClientError {
    fn from(error: ProxyError) -> Self {
        match error {
            ProxyError::NotFound => Self::not_found(),
            ProxyError::Upstream(_)
            | ProxyError::DigestMismatch { .. }
            | ProxyError::TooLarge { .. } => Self::new(StatusCode::BAD_GATEWAY, UPSTREAM_DETAIL),
            ProxyError::Unreachable(_) => {
                let (status, detail) = unreachable(&error);
                Self { status, detail }
            }
            ProxyError::Backend(_) => Self::internal(&error),
        }
    }
}

impl IntoResponse for ClientError {
    fn into_response(self) -> Response {
        let mut response = self.status.into_response();
        response
            .extensions_mut()
            .insert(ErrorBody::Detail(self.detail));
        response
    }
}

/// Диалект точки монтирования: определяет форму тела ошибки и схему аутентификации.
/// Ошибка формата не знает, как её покажут; слой `mark_dialect` на router формата кладёт диалект в ответ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dialect {
    Cargo,
    Npm,
}

/// Слой router'а формата (`from_fn_with_state(dialect, mark_dialect)`): снаружи auth, чтобы 401 и 403
/// тоже получили форму своего формата.
pub(crate) async fn mark_dialect(
    State(dialect): State<Dialect>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    response.extensions_mut().insert(dialect);
    response
}

/// Ошибка в форме RFC 9457 для маршрутов вне форматов.
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

impl Problem {
    pub(crate) const fn storage_unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            detail: "хранилище недоступно",
        }
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let mut response = self.status.into_response();
        response.extensions_mut().insert(ErrorBody::Problem {
            title: self.status.canonical_reason().unwrap_or("Error"),
            detail: self.detail,
        });
        response
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

/// 503: хранилище не отвечает, повтор позже имеет смысл. Это отказ среды, а не баг, поэтому warn.
fn unreachable(source: &(dyn Error + 'static)) -> (StatusCode, String) {
    // Цепочка может содержать хост хранилища: она только в журнале.
    tracing::warn!(error = %chain(source), "хранилище недоступно");
    (
        StatusCode::SERVICE_UNAVAILABLE,
        UNREACHABLE_DETAIL.to_owned(),
    )
}

fn describe(error: &RegistryError) -> (StatusCode, String) {
    let status = match error {
        RegistryError::RepositoryNotFound
        | RegistryError::VersionNotFound
        | RegistryError::TagNotFound
        | RegistryError::AssetNotFound => StatusCode::NOT_FOUND,
        RegistryError::NotHosted => StatusCode::METHOD_NOT_ALLOWED,
        RegistryError::VersionExists => StatusCode::CONFLICT,
        RegistryError::Unavailable {
            state: VersionState::Deleted,
        } => StatusCode::GONE,
        RegistryError::Unavailable { .. } | RegistryError::DeleteWindowClosed => {
            StatusCode::FORBIDDEN
        }
        RegistryError::DigestMismatch { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        RegistryError::Body(_) => StatusCode::BAD_REQUEST,
        RegistryError::Unreachable(_) => return unreachable(error),
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
    use std::sync::Arc;

    use crane_app::proxy::UpstreamFailure;
    use crane_domain::proxy::EgressRefusal;
    use crane_domain::registry::{Checksum, Sha256};

    use super::*;

    #[test]
    fn registry_errors_map_to_http_statuses() {
        // Arrange
        let digest = Checksum::Sha256(Sha256::from([1; 32]));
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
                    declared: Box::new(digest),
                    actual: Box::new(digest),
                },
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (RegistryError::DeleteWindowClosed, StatusCode::FORBIDDEN),
            (RegistryError::TagNotFound, StatusCode::NOT_FOUND),
            (
                RegistryError::Body(io::Error::other("обрыв")),
                StatusCode::BAD_REQUEST,
            ),
            (
                RegistryError::Unreachable("сбой".into()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                RegistryError::Backend("сбой".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        // Act
        let actual = cases.each_ref().map(|(error, _)| describe(error).0);

        // Assert
        assert_eq!(actual, cases.map(|(_, expected)| expected));
    }

    #[test]
    fn proxy_errors_map_to_status_and_hide_upstream_details() {
        // Arrange
        let sum = Checksum::Sha256(Sha256::from([1; 32]));
        let cases = [
            ProxyError::NotFound,
            ProxyError::Upstream(UpstreamFailure::Unavailable),
            ProxyError::Upstream(UpstreamFailure::Refused(EgressRefusal::HostNotAllowed(
                "evil.example".to_owned(),
            ))),
            ProxyError::DigestMismatch {
                expected: Box::new(sum),
                actual: Box::new(sum),
            },
            ProxyError::TooLarge { limit: 1 },
            ProxyError::Unreachable(Arc::new(io::Error::other("postgres://u:SECRET@h/db"))),
            ProxyError::Backend(Arc::new(io::Error::other("postgres://u:SECRET@h/db"))),
        ];

        // Act
        let actual = cases.map(|error| {
            let error = ClientError::from(error);
            (error.status, error.detail)
        });

        // Assert
        let bad_gateway = (StatusCode::BAD_GATEWAY, UPSTREAM_DETAIL.to_owned());
        assert_eq!(
            actual,
            [
                (StatusCode::NOT_FOUND, "не найдено".to_owned()),
                bad_gateway.clone(),
                bad_gateway.clone(),
                bad_gateway.clone(),
                bad_gateway,
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    UNREACHABLE_DETAIL.to_owned()
                ),
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    INTERNAL_DETAIL.to_owned()
                ),
            ]
        );
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
    fn unreachable_detail_hides_the_source_message() {
        // Arrange
        let error = RegistryError::Unreachable("postgres://u:SECRET@h/db".into());

        // Act
        let (_, detail) = describe(&error);

        // Assert
        assert_eq!(detail, UNREACHABLE_DETAIL);
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
