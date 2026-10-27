//! Временная статическая аутентификация (до v0.4.0).

use std::fmt;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::error::CargoError;

/// Статический токен: в памяти хранится только его SHA-256.
pub struct StaticToken {
    digest: [u8; 32],
}

impl StaticToken {
    #[must_use]
    pub fn from_secret(secret: &[u8]) -> Self {
        Self {
            digest: Sha256::digest(secret).into(),
        }
    }

    /// Сравнение дайджестов, а не сырых токенов: время не зависит ни от длины
    /// предъявленного значения, ни от длины общего префикса.
    pub(crate) fn verify(&self, presented: &[u8]) -> bool {
        let presented: [u8; 32] = Sha256::digest(presented).into();
        bool::from(presented.ct_eq(&self.digest))
    }
}

impl fmt::Debug for StaticToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StaticToken(..)")
    }
}

/// Cargo присылает токен в `Authorization` без схемы и не обязан слать валидный UTF-8,
/// поэтому заголовок читается как байты. Значение заголовка нигде не логируется.
pub(super) async fn require_token(
    State(token): State<Arc<StaticToken>>,
    request: Request,
    next: Next,
) -> Response {
    match request.headers().get(header::AUTHORIZATION) {
        // Cargo повторяет запрос к config.json с токеном только после 401.
        None => {
            let mut response =
                CargoError::new(StatusCode::UNAUTHORIZED, "требуется токен").into_response();
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Cargo"));
            response
        }
        Some(presented) if token.verify(presented.as_bytes()) => next.run(request).await,
        Some(_) => CargoError::new(StatusCode::FORBIDDEN, "токен отвергнут").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_accepts_only_the_exact_secret() {
        // Arrange
        let token = StaticToken::from_secret(b"s3cret");

        // Act
        let results = [
            token.verify(b"s3cret"),
            token.verify(b"s3cre"),
            token.verify(b"s3cret!"),
            token.verify(b""),
            token.verify(b"other!"),
        ];

        // Assert
        assert_eq!(results, [true, false, false, false, false]);
    }

    #[test]
    fn debug_does_not_reveal_digest() {
        // Arrange
        let token = StaticToken::from_secret(b"s3cret");

        // Act
        let shown = format!("{token:?}");

        // Assert
        assert_eq!(shown, "StaticToken(..)");
    }
}
