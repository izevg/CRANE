//! Временная статическая аутентификация (до v0.4.0).

mod credential;

use std::fmt;
use std::sync::Arc;

use axum::extract::{Extension, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use crane_domain::access::{Caller, Principal, RequestContext};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::onboarding::TokenHash;
use crane_domain::registry::Format;
use crane_ports::audit::AuditSink;

pub(crate) use credential::Credential;
use credential::{offered, presented};

use crate::context::Actor;
use crate::error::ClientError;
use crate::formats;

/// Статический токен: в памяти хранится только его SHA-256 и личность, которой он принадлежит.
pub struct StaticToken {
    hash: TokenHash,
    principal: Principal,
}

impl StaticToken {
    #[must_use]
    pub fn new(secret: &[u8], principal: Principal) -> Self {
        Self {
            hash: TokenHash::of(secret),
            principal,
        }
    }

    /// Сравнение дайджестов, а не сырых токенов: время не зависит ни от длины
    /// предъявленного значения, ни от длины общего префикса.
    pub(crate) fn authenticate(&self, presented: &[u8]) -> Option<&Principal> {
        self.hash.matches(presented).then_some(&self.principal)
    }
}

impl fmt::Debug for StaticToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StaticToken(..)")
    }
}

/// Состояние слоя аутентификации одного формата.
pub(crate) struct Auth {
    pub(crate) token: Arc<StaticToken>,
    pub(crate) audit: Arc<dyn AuditSink>,
    pub(crate) format: Format,
}

/// Заголовок читается как байты: cargo не обязан слать валидный UTF-8. Значение нигде не логируется.
pub(crate) async fn require_token(
    State(auth): State<Arc<Auth>>,
    Extension(context): Extension<RequestContext>,
    mut request: Request,
    next: Next,
) -> Response {
    if !offered(auth.format, request.headers()) {
        // Клиент всегда начинает с запроса без токена и повторяет его после 401: это
        // рукопожатие протокола, а не попытка входа, поэтому события аудита нет.
        let mut response =
            ClientError::new(StatusCode::UNAUTHORIZED, "требуется токен").into_response();
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, formats::challenge(auth.format));
        return response;
    }
    // Чужая схема даёт пустой секрет: он отвергается, а не принимается «как есть».
    let principal = {
        let presented = presented(auth.format, request.headers()).unwrap_or_default();
        auth.token.authenticate(&presented).cloned()
    };
    let Some(principal) = principal else {
        auth.audit.record(AuditEvent {
            actor: None,
            request: context,
            action: AuditAction::Authenticate,
            outcome: Outcome::Failure {
                reason: "токен отвергнут".to_owned(),
                code: None,
            },
        });
        return ClientError::new(StatusCode::FORBIDDEN, "токен отвергнут").into_response();
    };
    // Успешный вход не аудируется: 3002 Success — на настоящем событии входа в v0.4.0.
    let actor = Actor(principal.id.clone());
    request.extensions_mut().insert(Caller {
        principal,
        request: context,
    });
    let mut response = next.run(request).await;
    response.extensions_mut().insert(actor);
    response
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
