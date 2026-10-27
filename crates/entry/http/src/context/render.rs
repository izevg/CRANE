//! Тела ошибок рендерятся здесь и только здесь: id запроса известен лишь middleware.

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use crane_domain::access::RequestId;
use crane_format_cargo::render_errors;
use crane_format_npm::render_error as render_npm_error;
use serde_json::json;

use crate::error::{Dialect, ErrorBody};

/// Спека 04: клиенты и CI повторяют 503, окно failover PostgreSQL 10–30 с, поэтому просим подождать 10 с.
const RETRY_AFTER_SECONDS: HeaderValue = HeaderValue::from_static("10");

/// Тип проблемы (RFC 9457) для 503: отличает «хранилище недоступно, повторите» от общего `about:blank`.
const STORAGE_UNREACHABLE_TYPE: &str = "urn:crane:problem:storage-unreachable";
const GENERIC_TYPE: &str = "about:blank";

/// Подставляет тело вместо пустого, если ответ несёт `ErrorBody`; остальные ответы не трогает.
pub(super) fn render_error(mut response: Response, id: &RequestId) -> Response {
    let Some(error) = response.extensions_mut().remove::<ErrorBody>() else {
        return response;
    };
    let status = response.status();
    let dialect = response.extensions_mut().remove::<Dialect>();
    let (content_type, text) = match (error, dialect) {
        // Клиенты cargo и npm показывают пользователю только текст ошибки, поэтому id уходит суффиксом.
        (ErrorBody::Detail(detail), Some(Dialect::Cargo)) => (
            "application/json",
            render_errors(&format!("{detail} (request_id {})", id.as_str())),
        ),
        (ErrorBody::Detail(detail), Some(Dialect::Npm)) => (
            "application/json",
            render_npm_error(&format!("{detail} (request_id {})", id.as_str())),
        ),
        (ErrorBody::Detail(detail), None) => (
            "application/problem+json",
            problem(
                status,
                status.canonical_reason().unwrap_or("Error"),
                &detail,
                id,
            ),
        ),
        (ErrorBody::Problem { title, detail }, _) => (
            "application/problem+json",
            problem(status, title, detail, id),
        ),
    };
    *response.body_mut() = Body::from(text);
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    // Единственная точка: через неё проходят и форматы, и admin (включая /healthz/ready).
    if status == StatusCode::SERVICE_UNAVAILABLE {
        headers.insert(header::RETRY_AFTER, RETRY_AFTER_SECONDS);
    }
    response
}

/// json!, а не format!: detail и id могут содержать символы, требующие экранирования.
fn problem(status: StatusCode, title: &str, detail: &str, id: &RequestId) -> String {
    let kind = if status == StatusCode::SERVICE_UNAVAILABLE {
        STORAGE_UNREACHABLE_TYPE
    } else {
        GENERIC_TYPE
    };
    json!({
        "type": kind,
        "title": title,
        "status": status.as_u16(),
        "detail": detail,
        "request_id": id.as_str(),
    })
    .to_string()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    use super::*;

    fn rendered(dialect: Option<Dialect>) -> (String, String) {
        let response = render_with(StatusCode::BAD_REQUEST, dialect);
        let content_type = response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_owned();
        (content_type, body_text(response))
    }

    fn render_with(status: StatusCode, dialect: Option<Dialect>) -> Response {
        let mut response = status.into_response();
        response
            .extensions_mut()
            .insert(ErrorBody::Detail("плохой запрос".to_owned()));
        if let Some(dialect) = dialect {
            response.extensions_mut().insert(dialect);
        }
        let id = RequestId::parse("req-1").unwrap();
        render_error(response, &id)
    }

    fn body_text(response: Response) -> String {
        let body = futures_util::FutureExt::now_or_never(response.into_body().collect())
            .unwrap()
            .unwrap()
            .to_bytes();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[test]
    fn service_unavailable_asks_clients_to_retry_and_other_errors_do_not() {
        // Arrange
        let statuses = [
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
        ];

        // Act
        let actual = statuses.map(|status| {
            render_with(status, Some(Dialect::Cargo))
                .headers()
                .get(header::RETRY_AFTER)
                .cloned()
        });

        // Assert
        assert_eq!(actual, [Some(HeaderValue::from_static("10")), None, None]);
    }

    #[test]
    fn service_unavailable_problem_has_its_own_type_and_cargo_body_is_unchanged() {
        // Arrange
        let dialects = [None, Some(Dialect::Cargo)];

        // Act
        let actual = dialects
            .map(|dialect| body_text(render_with(StatusCode::SERVICE_UNAVAILABLE, dialect)));

        // Assert
        let [problem, cargo] = actual;
        assert_eq!(
            (problem.contains(STORAGE_UNREACHABLE_TYPE), cargo),
            (
                true,
                r#"{"errors":[{"detail":"плохой запрос (request_id req-1)"}]}"#.to_owned()
            )
        );
    }

    #[test]
    fn detail_takes_the_shape_of_the_dialect() {
        // Arrange
        let dialects = [Some(Dialect::Cargo), Some(Dialect::Npm), None];

        // Act
        let actual = dialects.map(rendered);

        // Assert
        let [cargo, npm, none] = actual;
        assert_eq!(
            (cargo, npm.0, npm.1),
            (
                (
                    "application/json".to_owned(),
                    r#"{"errors":[{"detail":"плохой запрос (request_id req-1)"}]}"#.to_owned()
                ),
                "application/json".to_owned(),
                r#"{"error":"плохой запрос (request_id req-1)"}"#.to_owned()
            )
        );
        assert_eq!(none.0, "application/problem+json");
    }
}
