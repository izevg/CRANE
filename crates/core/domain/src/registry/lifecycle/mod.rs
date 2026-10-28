use std::time::{Duration, SystemTime};

use crate::proxy::age;

/// Состояние версии (SQL enum `ver_state`). `Staging` — состояние загрузки, не версии.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VersionState {
    Quarantined,
    Released,
    Blocked,
    Deleted,
}

/// Запрещённый переход между состояниями версии.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("переход {from:?} -> {to:?} запрещён")]
pub struct TransitionError {
    pub from: VersionState,
    pub to: VersionState,
}

impl VersionState {
    /// Рёбра рис. 9.2. `Deleted` терминально: номер релиза занять повторно нельзя.
    #[must_use]
    pub const fn can_become(self, to: Self) -> bool {
        matches!(
            (self, to),
            (
                Self::Quarantined,
                Self::Released | Self::Blocked | Self::Deleted
            ) | (Self::Released, Self::Blocked | Self::Deleted)
                | (Self::Blocked, Self::Released)
        )
    }

    pub const fn ensure(self, to: Self) -> Result<(), TransitionError> {
        if self.can_become(to) {
            Ok(())
        } else {
            Err(TransitionError { from: self, to })
        }
    }
}

/// Политика удаления: опубликованную версию можно удалить только в течение `window`.
/// Дальше остаётся только блокировка (Blocked): ломать зависимых нельзя.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletePolicy {
    pub window: Duration,
}

impl DeletePolicy {
    /// Возраст ровно `window` ещё допустим. `published_at` в будущем (расхождение часов) даёт возраст 0.
    #[must_use]
    pub fn permits(&self, published_at: SystemTime, now: SystemTime) -> bool {
        age(published_at, now) <= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::VersionState::{Blocked, Deleted, Quarantined, Released};
    use super::*;

    #[test]
    fn transition_matrix_matches_lifecycle_diagram() {
        // Arrange
        let all = [Quarantined, Released, Blocked, Deleted];
        // Строки — from, столбцы — to, порядок как в `all`.
        let expected = [
            [false, true, true, true],
            [false, false, true, true],
            [false, true, false, false],
            [false, false, false, false],
        ];

        // Act
        let actual = all.map(|from| all.map(|to| from.can_become(to)));

        // Assert
        assert_eq!(actual, expected);
    }

    #[test]
    fn ensure_accepts_allowed_transition() {
        // Arrange
        let (from, to) = (Quarantined, Released);

        // Act
        let result = from.ensure(to);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn ensure_reports_both_states_of_forbidden_transition() {
        // Arrange
        let (from, to) = (Deleted, Released);

        // Act
        let result = from.ensure(to);

        // Assert
        assert_eq!(
            result,
            Err(TransitionError {
                from: Deleted,
                to: Released
            })
        );
    }

    #[test]
    fn delete_policy_permits_up_to_and_including_window() {
        // Arrange
        let window = Duration::from_secs(100);
        let policy = DeletePolicy { window };
        let published = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let cases = [
            Duration::ZERO,
            Duration::from_secs(99),
            window,
            Duration::from_secs(101),
        ];

        // Act
        let actual = cases.map(|elapsed| policy.permits(published, published + elapsed));

        // Assert
        assert_eq!(actual, [true, true, true, false]);
    }

    #[test]
    fn delete_policy_treats_future_publication_as_age_zero() {
        // Arrange
        let policy = DeletePolicy {
            window: Duration::ZERO,
        };
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let published = now + Duration::from_secs(60);

        // Act
        let permitted = policy.permits(published, now);

        // Assert
        assert!(permitted);
    }
}
