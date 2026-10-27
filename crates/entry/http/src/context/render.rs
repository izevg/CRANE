//! Тела ошибок рендерятся здесь и только здесь: id запроса известен лишь middleware.

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use crane_domain::access::RequestId;
use crane_format_cargo::render_errors;
use crane_format_npm::render_error as render_npm_error;
use serde_json::json;

use crate::error::{Dialect, ErrorBody};

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
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}

/// json!, а не format!: detail и id могут содержать символы, требующие экранирования.
fn problem(status: StatusCode, title: &str, detail: &str, id: &RequestId) -> String {
    json!({
        "type": "about:blank",
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
        let mut response = StatusCode::BAD_REQUEST.into_response();
        response
            .extensions_mut()
            .insert(ErrorBody::Detail("плохой запрос".to_owned()));
        if let Some(dialect) = dialect {
            response.extensions_mut().insert(dialect);
        }
        let id = RequestId::parse("req-1").unwrap();
        let response = render_error(response, &id);
        let content_type = response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_owned();
        let body = futures_util::FutureExt::now_or_never(response.into_body().collect())
            .unwrap()
            .unwrap()
            .to_bytes();
        (content_type, String::from_utf8(body.to_vec()).unwrap())
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
