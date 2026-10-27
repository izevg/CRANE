//! Тела ошибок рендерятся здесь и только здесь: id запроса известен лишь middleware.

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use crane_domain::access::RequestId;
use crane_format_cargo::render_errors;
use crane_format_npm::render_error as render_npm_error;
use serde_json::json;

use crate::error::{Dialect, ErrorBody, ProblemKind};

/// Спека 04: клиенты и CI повторяют 503, окно failover PostgreSQL 10–30 с, поэтому просим подождать 10 с.
const RETRY_AFTER_SECONDS: HeaderValue = HeaderValue::from_static("10");

/// Подставляет тело вместо пустого, если ответ несёт `ErrorBody`; остальные ответы не трогает.
pub(super) fn render_error(mut response: Response, id: &RequestId) -> Response {
    let Some(error) = response.extensions_mut().remove::<ErrorBody>() else {
        return response;
    };
    let status = response.status();
    let dialect = response.extensions_mut().remove::<Dialect>();
    let (content_type, text) = match (error, dialect) {
        // Клиенты cargo и npm показывают пользователю только текст ошибки, поэтому id уходит суффиксом.
        (ErrorBody::Detail { detail, .. }, Some(Dialect::Cargo)) => (
            "application/json",
            render_errors(&format!("{detail} (request_id {})", id.as_str())),
        ),
        (ErrorBody::Detail { detail, .. }, Some(Dialect::Npm)) => (
            "application/json",
            render_npm_error(&format!("{detail} (request_id {})", id.as_str())),
        ),
        (ErrorBody::Detail { detail, kind }, None) => (
            "application/problem+json",
            problem(
                status,
                status.canonical_reason().unwrap_or("Error"),
                &detail,
                kind,
                id,
            ),
        ),
        (
            ErrorBody::Problem {
                title,
                detail,
                kind,
            },
            _,
        ) => (
            "application/problem+json",
            problem(status, title, detail, kind, id),
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
fn problem(
    status: StatusCode,
    title: &str,
    detail: &str,
    kind: ProblemKind,
    id: &RequestId,
) -> String {
    json!({
        "type": kind.uri(),
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
        let response = render_with(StatusCode::BAD_REQUEST, dialect, ProblemKind::Blank);
        let content_type = response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_owned();
        (content_type, body_text(response))
    }

    fn render_with(status: StatusCode, dialect: Option<Dialect>, kind: ProblemKind) -> Response {
        let mut response = status.into_response();
        response.extensions_mut().insert(ErrorBody::Detail {
            detail: "плохой запрос".to_owned(),
            kind,
        });
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
            render_with(status, Some(Dialect::Cargo), ProblemKind::Blank)
                .headers()
                .get(header::RETRY_AFTER)
                .cloned()
        });

        // Assert
        assert_eq!(actual, [Some(HeaderValue::from_static("10")), None, None]);
    }

    #[test]
    fn problem_type_follows_the_kind_of_the_error_not_the_status() {
        // Arrange
        let kinds = [ProblemKind::StorageUnreachable, ProblemKind::Blank];

        // Act
        let actual =
            kinds.map(|kind| body_text(render_with(StatusCode::SERVICE_UNAVAILABLE, None, kind)));

        // Assert
        let types = actual
            .map(|body| serde_json::from_str::<serde_json::Value>(&body).unwrap()["type"].clone());
        assert_eq!(
            types,
            [
                serde_json::json!("urn:crane:problem:storage-unreachable"),
                serde_json::json!("about:blank")
            ]
        );
    }

    #[test]
    fn cargo_body_of_an_unreachable_error_keeps_the_dialect_shape() {
        // Arrange
        let kind = ProblemKind::StorageUnreachable;

        // Act
        let body = body_text(render_with(
            StatusCode::SERVICE_UNAVAILABLE,
            Some(Dialect::Cargo),
            kind,
        ));

        // Assert
        assert_eq!(
            body,
            r#"{"errors":[{"detail":"плохой запрос (request_id req-1)"}]}"#
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
