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
use crate::error::{ClientError, Dialect};

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

const BEARER: &[u8] = b"Bearer ";

impl Dialect {
    fn challenge(self) -> HeaderValue {
        match self {
            Self::Cargo => HeaderValue::from_static("Cargo"),
            Self::Npm => HeaderValue::from_static("Bearer realm=\"crane\""),
        }
    }

    /// Cargo присылает токен без схемы; npm — `Bearer <токен>` (схема без учёта регистра, один пробел).
    /// Чужая схема даёт пустой токен: он отвергается, а не принимается «как есть».
    fn token(self, header: &[u8]) -> &[u8] {
        match self {
            Self::Cargo => header,
            Self::Npm => match header.split_at_checked(BEARER.len()) {
                Some((scheme, token)) if scheme.eq_ignore_ascii_case(BEARER) => token,
                _ => &[],
            },
        }
    }
}

/// Состояние слоя аутентификации одного диалекта.
pub(crate) struct Auth {
    pub(crate) token: Arc<StaticToken>,
    pub(crate) audit: Arc<dyn AuditSink>,
    pub(crate) dialect: Dialect,
}

/// Заголовок читается как байты: cargo не обязан слать валидный UTF-8. Значение нигде не логируется.
pub(crate) async fn require_token(
    State(auth): State<Arc<Auth>>,
    Extension(context): Extension<RequestContext>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(presented) = request.headers().get(header::AUTHORIZATION) else {
        // Cargo всегда начинает с запроса без токена и повторяет его после 401: это
        // рукопожатие протокола, а не попытка входа, поэтому события аудита нет.
        let mut response =
            ClientError::new(StatusCode::UNAUTHORIZED, "требуется токен").into_response();
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, auth.dialect.challenge());
        return response;
    };
    let Some(principal) = auth
        .token
        .authenticate(auth.dialect.token(presented.as_bytes()))
    else {
        auth.audit.record(AuditEvent {
            actor: None,
            request: context,
            action: AuditAction::Authenticate,
            outcome: Outcome::Failure {
                reason: "токен отвергнут".to_owned(),
            },
        });
        return ClientError::new(StatusCode::FORBIDDEN, "токен отвергнут").into_response();
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
    fn dialect_extracts_token_from_authorization_value() {
        // Arrange
        let cases: [(Dialect, &[u8]); 8] = [
            (Dialect::Cargo, b"tok"),
            (Dialect::Cargo, b"Bearer tok"),
            (Dialect::Npm, b"Bearer tok"),
            (Dialect::Npm, b"bearer tok"),
            (Dialect::Npm, b"BEARER tok"),
            (Dialect::Npm, b"tok"),
            (Dialect::Npm, b"Basic tok"),
            (Dialect::Npm, b"Bearer"),
        ];

        // Act
        let actual = cases.map(|(dialect, header)| dialect.token(header));

        // Assert
        let expected: [&[u8]; 8] = [b"tok", b"Bearer tok", b"tok", b"tok", b"tok", b"", b"", b""];
        assert_eq!(actual, expected);
    }

    #[test]
    fn dialect_challenge_names_the_scheme_of_the_format() {
        // Arrange
        let dialects = [Dialect::Cargo, Dialect::Npm];

        // Act
        let actual = dialects.map(Dialect::challenge);

        // Assert
        assert_eq!(
            actual,
            [
                HeaderValue::from_static("Cargo"),
                HeaderValue::from_static("Bearer realm=\"crane\"")
            ]
        );
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
