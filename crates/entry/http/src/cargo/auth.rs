//! Временная статическая аутентификация (до v0.4.0).

use std::fmt;
use std::sync::Arc;

use axum::extract::{Extension, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use crane_domain::access::{Caller, Principal, RequestContext};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_ports::audit::AuditSink;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::context::Actor;
use crate::error::CargoError;

/// Статический токен: в памяти хранится только его SHA-256 и личность, которой он принадлежит.
pub struct StaticToken {
    digest: [u8; 32],
    principal: Principal,
}

impl StaticToken {
    #[must_use]
    pub fn new(secret: &[u8], principal: Principal) -> Self {
        Self {
            digest: Sha256::digest(secret).into(),
            principal,
        }
    }

    /// Сравнение дайджестов, а не сырых токенов: время не зависит ни от длины
    /// предъявленного значения, ни от длины общего префикса.
    pub(crate) fn authenticate(&self, presented: &[u8]) -> Option<&Principal> {
        let presented: [u8; 32] = Sha256::digest(presented).into();
        bool::from(presented.ct_eq(&self.digest)).then_some(&self.principal)
    }
}

impl fmt::Debug for StaticToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StaticToken(..)")
    }
}

/// Состояние слоя аутентификации.
pub(super) struct Auth {
    pub(super) token: StaticToken,
    pub(super) audit: Arc<dyn AuditSink>,
}

/// Cargo присылает токен в `Authorization` без схемы и не обязан слать валидный UTF-8,
/// поэтому заголовок читается как байты. Значение заголовка нигде не логируется.
pub(super) async fn require_token(
    State(auth): State<Arc<Auth>>,
    Extension(context): Extension<RequestContext>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(presented) = request.headers().get(header::AUTHORIZATION) else {
        // Cargo всегда начинает с запроса без токена и повторяет его после 401: это
        // рукопожатие протокола, а не попытка входа, поэтому события аудита нет.
        let mut response =
            CargoError::new(StatusCode::UNAUTHORIZED, "требуется токен").into_response();
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Cargo"));
        return response;
    };
    let Some(principal) = auth.token.authenticate(presented.as_bytes()) else {
        auth.audit.record(AuditEvent {
            actor: None,
            request: context,
            action: AuditAction::Authenticate,
            outcome: Outcome::Failure {
                reason: "токен отвергнут".to_owned(),
            },
        });
        return CargoError::new(StatusCode::FORBIDDEN, "токен отвергнут").into_response();
    };
    // Успешный вход не аудируется: 3002 Success — на настоящем событии входа в v0.4.0.
    let actor = Actor(principal.id.clone());
    request.extensions_mut().insert(Caller {
        principal: principal.clone(),
        request: context,
    });
    let mut response = next.run(request).await;
    response.extensions_mut().insert(actor);
    response
}

#[cfg(test)]
mod tests {
    use crane_domain::access::PrincipalKind;

    use super::*;

    fn token() -> StaticToken {
        StaticToken::new(
            b"s3cret",
            Principal {
                id: "ci".to_owned(),
                kind: PrincipalKind::Robot,
            },
        )
    }

    #[test]
    fn authenticate_accepts_only_the_exact_secret() {
        // Arrange
        let token = token();
        let presented: [&[u8]; 5] = [b"s3cret", b"s3cre", b"s3cret!", b"", b"other!"];

        // Act
        let accepted = presented.map(|value| token.authenticate(value).map(|p| p.id.as_str()));

        // Assert
        assert_eq!(accepted, [Some("ci"), None, None, None, None]);
    }
}
