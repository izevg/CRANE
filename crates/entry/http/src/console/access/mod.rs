//! Аутентификация консоли: `Authorization: Bearer <CRANE_TOKEN>` против того же `StaticToken`, что у протоколов.

use std::sync::Arc;

use axum::extract::{Extension, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use crane_domain::access::{Caller, RequestContext};
use crane_ports::audit::AuditSink;

use crate::auth::{Credential, StaticToken, record_rejected};
use crate::context::Actor;
use crate::error::ClientError;

/// Без токена доступна только публичная часть статуса (версия, режим, фаза).
pub(super) const PUBLIC_STATUS: &str = "/api/v1/status";
const CHALLENGE: HeaderValue = HeaderValue::from_static("Bearer");

pub(super) struct Access {
    pub(super) token: StaticToken,
    pub(super) audit: Arc<dyn AuditSink>,
}

/// Верный токен кладёт `Caller` в расширения запроса. Любой другой исход, кроме анонимного чтения
/// публичного статуса, — 401 и событие `Authenticate` с отказом: у Bearer-API нет рукопожатия,
/// которое отличало бы первый запрос без токена от попытки входа.
pub(super) async fn require_bearer(
    State(access): State<Arc<Access>>,
    Extension(context): Extension<RequestContext>,
    mut request: Request,
    next: Next,
) -> Response {
    let anonymous = !request.headers().contains_key(header::AUTHORIZATION);
    if anonymous && is_public(&request) {
        return next.run(request).await;
    }
    // Значение заголовка нигде не логируется; чужая схема даёт `None`, а не «токен как есть».
    let principal = Credential::Bearer
        .extract(request.headers())
        .and_then(|presented| access.token.authenticate(&presented).cloned());
    let Some(principal) = principal else {
        let detail = if anonymous {
            "требуется токен"
        } else {
            "токен отвергнут"
        };
        record_rejected(access.audit.as_ref(), context);
        let mut response = ClientError::new(StatusCode::UNAUTHORIZED, detail).into_response();
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, CHALLENGE);
        return response;
    };
    let actor = Actor(principal.id.clone());
    request.extensions_mut().insert(Caller {
        principal,
        request: context,
    });
    let mut response = next.run(request).await;
    response.extensions_mut().insert(actor);
    response
}

fn is_public(request: &Request) -> bool {
    request.method() == Method::GET && request.uri().path() == PUBLIC_STATUS
}
