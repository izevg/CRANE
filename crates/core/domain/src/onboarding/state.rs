use std::time::SystemTime;

use super::TokenHash;
use crate::settings::Secret;

/// Состояние мастера первого запуска. Два состояния: «завершён, но с токеном» непредставимо.
/// Без `Debug`: внутри хэш токена.
#[allow(
    missing_debug_implementations,
    reason = "внутри хэш токена, Debug запрещён (14.9)"
)]
#[derive(Clone)]
pub enum Onboarding {
    Pending {
        token: TokenHash,
        expires_at: SystemTime,
    },
    Completed {
        at: SystemTime,
        by: String,
    },
}

impl Onboarding {
    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    /// Принимает токен, только пока мастер открыт, срок не вышел и хэш совпал.
    #[must_use]
    pub fn accepts(&self, presented: &Secret, now: SystemTime) -> bool {
        match self {
            // На `expires_at` токен уже не действует.
            Self::Pending { token, expires_at } => now < *expires_at && token.matches(presented),
            Self::Completed { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const NOW: SystemTime = SystemTime::UNIX_EPOCH;

    fn pending(token: &str, ttl: Duration) -> Onboarding {
        Onboarding::Pending {
            token: TokenHash::of(&Secret::new(token.to_owned())),
            expires_at: NOW + ttl,
        }
    }

    #[test]
    fn accepts_only_valid_unexpired_token_of_pending_state() {
        // Arrange
        let second = Duration::from_secs(1);
        let completed = Onboarding::Completed {
            at: NOW,
            by: "crane:system".to_owned(),
        };
        let cases = [
            (pending("crb_ok", second), "crb_ok", NOW),
            (pending("crb_ok", second), "crb_ok", NOW + second),
            (pending("crb_ok", second), "crb_ok", NOW + second * 2),
            (pending("crb_ok", second), "crb_bad", NOW),
            (completed, "crb_ok", NOW),
        ];

        // Act
        let actual =
            cases.map(|(state, raw, now)| state.accepts(&Secret::new(raw.to_owned()), now));

        // Assert
        assert_eq!(actual, [true, false, false, false, false]);
    }

    #[test]
    fn is_completed_is_true_only_for_completed() {
        // Arrange
        let cases = [
            pending("crb_ok", Duration::from_secs(1)),
            Onboarding::Completed {
                at: NOW,
                by: "x".to_owned(),
            },
        ];

        // Act
        let actual = cases.map(|state| state.is_completed());

        // Assert
        assert_eq!(actual, [false, true]);
    }
}
