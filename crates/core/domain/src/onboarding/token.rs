use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::settings::Secret;

/// SHA-256 bootstrap-токена. В БД лежит только он. Без `Debug` (хэш токена не выводится, 14.9)
/// и без `PartialEq`: обычное сравнение не в постоянное время, сравнивать нужно через `matches`.
#[allow(
    missing_debug_implementations,
    reason = "Debug хэша токена запрещён (14.9)"
)]
#[derive(Clone)]
pub struct TokenHash([u8; 32]);

impl TokenHash {
    #[must_use]
    pub fn of(token: &Secret) -> Self {
        Self(Sha256::digest(token.expose().as_bytes()).into())
    }

    /// Для адаптеров, читающих хэш из хранилища.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Сравнивает хэши, а не токены, в постоянное время: время ответа не выдаёт длину общего префикса.
    #[must_use]
    pub fn matches(&self, presented: &Secret) -> bool {
        self.0.ct_eq(&Self::of(presented).0).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_accepts_same_token_and_rejects_other() {
        // Arrange
        let hash = TokenHash::of(&Secret::new("crb_alpha".to_owned()));
        let cases = ["crb_alpha", "crb_alphb", ""];

        // Act
        let actual = cases.map(|raw| hash.matches(&Secret::new(raw.to_owned())));

        // Assert
        assert_eq!(actual, [true, false, false]);
    }
}
