//! Тела ошибок рендерятся здесь и только здесь: id запроса известен лишь middleware.

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use crane_domain::access::RequestId;
use crane_domain::registry::Format;
use serde_json::{Map, Value, json};

use crate::error::{ErrorBody, ProblemKind};
use crate::formats;

/// Спека 04: клиенты и CI повторяют 503, окно failover PostgreSQL 10–30 с, поэтому просим подождать 10 с.
const RETRY_AFTER_SECONDS: HeaderValue = HeaderValue::from_static("10");
const PROBLEM_JSON: &str = "application/problem+json";

/// Подставляет тело вместо пустого, если ответ несёт `ErrorBody`; остальные ответы не трогает.
pub(super) fn render_error(mut response: Response, id: &RequestId) -> Response {
    let Some(error) = response.extensions_mut().remove::<ErrorBody>() else {
        return response;
    };
    let status = response.status();
    let format = response.extensions_mut().remove::<Format>();
    let (content_type, text) = match error {
        ErrorBody::Detail { detail, kind } => format
            .and_then(|format| formats::error_body(format, &detail, id.as_str()))
            .unwrap_or_else(|| {
                let title = status.canonical_reason().unwrap_or("Error");
                (
                    PROBLEM_JSON,
                    problem(status, title, &detail, kind, id, &Map::new()),
                )
            }),
        ErrorBody::Problem {
            title,
            detail,
            kind,
            extensions,
        } => (
            PROBLEM_JSON,
            problem(status, title, &detail, kind, id, &extensions),
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
/// Расширения добавляются последними и стандартные члены не заменяют.
fn problem(
    status: StatusCode,
    title: &str,
    detail: &str,
    kind: ProblemKind,
    id: &RequestId,
    extensions: &Map<String, Value>,
) -> String {
    let mut body = json!({
        "type": kind.uri(),
        "title": title,
        "status": status.as_u16(),
        "detail": detail,
        "request_id": id.as_str(),
    });
    if let Some(members) = body.as_object_mut() {
        for (name, value) in extensions {
            members.entry(name).or_insert_with(|| value.clone());
        }
    }
    body.to_string()
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

    fn rendered(format: Option<Format>) -> (String, String) {
        let response = render_with(StatusCode::BAD_REQUEST, format, ProblemKind::Blank);
        let content_type = response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_owned();
        (content_type, body_text(response))
    }

    fn render_with(status: StatusCode, format: Option<Format>, kind: ProblemKind) -> Response {
        let mut response = status.into_response();
        response.extensions_mut().insert(ErrorBody::Detail {
            detail: "плохой запрос".to_owned(),
            kind,
        });
        if let Some(format) = format {
            response.extensions_mut().insert(format);
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
            render_with(status, Some(Format::Cargo), ProblemKind::Blank)
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
    fn cargo_body_of_an_unreachable_error_keeps_the_format_shape() {
        // Arrange
        let kind = ProblemKind::StorageUnreachable;

        // Act
        let body = body_text(render_with(
            StatusCode::SERVICE_UNAVAILABLE,
            Some(Format::Cargo),
            kind,
        ));

        // Assert
        assert_eq!(
            body,
            r#"{"errors":[{"detail":"плохой запрос (request_id req-1)"}]}"#
        );
    }

    #[test]
    fn detail_takes_the_shape_of_the_format() {
        // Arrange
        let formats = [
            Some(Format::Cargo),
            Some(Format::Npm),
            None,
            Some(Format::PyPi),
        ];

        // Act
        let actual = formats.map(rendered);

        // Assert
        let [cargo, npm, none, pypi] = actual;
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
        assert_eq!(
            [none.0, pypi.0],
            ["application/problem+json", "application/problem+json"]
        );
    }
}
