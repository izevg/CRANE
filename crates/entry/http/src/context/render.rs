//! Тела ошибок рендерятся здесь и только здесь: id запроса известен лишь middleware.

use axum::body::Body;
use axum::http::{HeaderValue, header};
use axum::response::Response;
use crane_domain::access::RequestId;
use crane_format_cargo::render_errors;
use serde_json::json;

use crate::error::ErrorBody;

/// Подставляет тело вместо пустого, если ответ несёт `ErrorBody`; остальные ответы не трогает.
pub(super) fn render_error(mut response: Response, id: &RequestId) -> Response {
    let Some(error) = response.extensions_mut().remove::<ErrorBody>() else {
        return response;
    };
    let status = response.status();
    let (content_type, text) = match error {
        // Cargo показывает пользователю только `detail`, поэтому id уходит суффиксом.
        ErrorBody::Cargo(detail) => (
            "application/json",
            render_errors(&format!("{detail} (request_id {})", id.as_str())),
        ),
        // json!, а не format!: detail и id могут содержать символы, требующие экранирования.
        ErrorBody::Problem { title, detail } => (
            "application/problem+json",
            json!({
                "type": "about:blank",
                "title": title,
                "status": status.as_u16(),
                "detail": detail,
                "request_id": id.as_str(),
            })
            .to_string(),
        ),
    };
    *response.body_mut() = Body::from(text);
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}
