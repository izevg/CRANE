use std::num::NonZeroU32;
use std::time::{Duration, SystemTime};

/// Потолок множителя адаптивного TTL: 2^3 = ×8 (docs/product/11-heuristics.md, 11.1).
const ADAPTIVE_MAX_DOUBLINGS: u32 = 3;

/// Возраст записи. Если часы реплик разошлись и `fetched_at` оказался в будущем, возраст 0:
/// запись считается свежей, а не вечно «из будущего».
#[must_use]
pub fn age(fetched_at: SystemTime, now: SystemTime) -> Duration {
    now.duration_since(fetched_at).unwrap_or(Duration::ZERO)
}

/// Отрицательный кэш с ростом: повторные 404 одного ключа удваивают срок до `max`.
/// Защищает upstream от опечаток в CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NegativePolicy {
    pub ttl: Duration,
    pub max: Duration,
}

impl NegativePolicy {
    /// Артефакты всех форматов (11.1): 5 мин, рост до 1 ч.
    pub const ARTIFACT: Self = Self {
        ttl: Duration::from_secs(300),
        max: Duration::from_secs(3600),
    };

    /// Срок отрицательной записи после `misses` подряд 404: `ttl · 2^(misses−1)`, не больше `max`.
    /// `misses = 0` трактуется как 1: запись уже существует, значит один промах был.
    #[must_use]
    pub fn ttl_after(&self, misses: u32) -> Duration {
        let doublings = misses.saturating_sub(1);
        // Сдвиг на 32 и больше не представим в u32: это заведомо выше любого разумного max.
        let factor = 1_u32.checked_shl(doublings);
        factor
            .map_or(self.max, |factor| self.ttl.saturating_mul(factor))
            .min(self.max)
    }

    #[must_use]
    pub fn is_expired(&self, misses: u32, fetched_at: SystemTime, now: SystemTime) -> bool {
        age(fetched_at, now) >= self.ttl_after(misses)
    }
}

/// Политика кэша метаданных. Работает над фактами записи (когда получена, сколько раз не менялась);
/// тело и `ETag` хранятся в портах.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetadataPolicy {
    pub ttl: Duration,
    /// Окно stale-while-revalidate после истечения TTL; по умолчанию равно `ttl` (11.1).
    pub revalidate_window: Duration,
    pub stale_if_error: Duration,
    /// N из 11.1: сколько необновлений подряд поднимают TTL на ступень.
    pub adaptive_after: NonZeroU32,
    pub negative: NegativePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Отдать как есть.
    Fresh,
    /// Отдать и обновить в фоне (refresh-ahead в последних 10% TTL, затем stale-while-revalidate).
    Revalidate,
    /// Идти в upstream блокирующим запросом.
    Expired,
}

impl MetadataPolicy {
    /// Адаптивный TTL: `ttl · 2^min(unchanged / adaptive_after, 3)`, потолок ×8 (11.1).
    #[must_use]
    pub fn effective_ttl(&self, unchanged: u32) -> Duration {
        let doublings = (unchanged / self.adaptive_after.get()).min(ADAPTIVE_MAX_DOUBLINGS);
        self.ttl.saturating_mul(1 << doublings)
    }

    /// Зоны по возрасту записи. Границы целочисленные (без f64): нижняя — `eff − eff/10`, то есть последние 10% TTL.
    #[must_use]
    pub fn assess(&self, unchanged: u32, fetched_at: SystemTime, now: SystemTime) -> Freshness {
        let effective = self.effective_ttl(unchanged);
        let age = age(fetched_at, now);
        if age < effective.saturating_sub(effective / 10) {
            Freshness::Fresh
        } else if age < effective.saturating_add(self.revalidate_window) {
            Freshness::Revalidate
        } else {
            Freshness::Expired
        }
    }

    /// Запись ещё годится при сбое upstream.
    #[must_use]
    pub fn usable_on_error(&self, unchanged: u32, fetched_at: SystemTime, now: SystemTime) -> bool {
        age(fetched_at, now)
            < self
                .effective_ttl(unchanged)
                .saturating_add(self.stale_if_error)
    }

    /// Сколько хранить запись (TTL ключа L2): пока она может понадобиться как stale или как резерв при сбое.
    #[must_use]
    pub fn keep_for(&self, unchanged: u32) -> Duration {
        self.effective_ttl(unchanged)
            .saturating_add(self.revalidate_window.max(self.stale_if_error))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    /// Момент за 1 мс до границы.
    fn ms_before(boundary: Duration) -> Duration {
        boundary.saturating_sub(MS)
    }

    fn policy() -> MetadataPolicy {
        MetadataPolicy {
            ttl: Duration::from_secs(100),
            revalidate_window: Duration::from_secs(100),
            stale_if_error: Duration::from_secs(1000),
            adaptive_after: NonZeroU32::new(3).unwrap(),
            negative: NegativePolicy::ARTIFACT,
        }
    }

    fn fetched() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000)
    }

    #[test]
    fn effective_ttl_doubles_per_adaptive_step_up_to_eight_times() {
        // Arrange
        let policy = policy();
        let cases = [
            (0, 1),
            (2, 1),
            (3, 2),
            (5, 2),
            (6, 4),
            (9, 8),
            (12, 8),
            (100, 8),
            (u32::MAX, 8),
        ];

        // Act
        let ttls = cases.map(|(unchanged, _)| policy.effective_ttl(unchanged));

        // Assert
        assert_eq!(ttls, cases.map(|(_, factor)| policy.ttl * factor));
    }

    #[test]
    fn effective_ttl_does_not_overflow_on_huge_base() {
        // Arrange
        let mut policy = policy();
        policy.ttl = Duration::MAX;

        // Act
        let ttl = policy.effective_ttl(100);

        // Assert
        assert_eq!(ttl, Duration::MAX);
    }

    #[test]
    fn assess_switches_zones_exactly_on_boundaries() {
        // Arrange
        let policy = policy();
        let fresh_until = Duration::from_secs(90);
        let expired_from = Duration::from_secs(200);
        let cases = [
            (Duration::ZERO, Freshness::Fresh),
            (ms_before(fresh_until), Freshness::Fresh),
            (fresh_until, Freshness::Revalidate),
            (ms_before(expired_from), Freshness::Revalidate),
            (expired_from, Freshness::Expired),
            (Duration::from_secs(100_000), Freshness::Expired),
        ];

        // Act
        let zones = cases.map(|(age, _)| policy.assess(0, fetched(), fetched() + age));

        // Assert
        assert_eq!(zones, cases.map(|(_, zone)| zone));
    }

    #[test]
    fn assess_shifts_zones_with_adaptive_ttl() {
        // Arrange
        let policy = policy();
        let unchanged = 3; // eff = 200 с: Fresh до 180 с, Expired с 300 с
        let cases = [
            (Duration::from_secs(179), Freshness::Fresh),
            (Duration::from_secs(180), Freshness::Revalidate),
            (Duration::from_secs(299), Freshness::Revalidate),
            (Duration::from_secs(300), Freshness::Expired),
        ];

        // Act
        let zones = cases.map(|(age, _)| policy.assess(unchanged, fetched(), fetched() + age));

        // Assert
        assert_eq!(zones, cases.map(|(_, zone)| zone));
    }

    #[test]
    fn assess_without_revalidate_window_goes_straight_to_expired_at_ttl() {
        // Arrange
        let mut policy = policy();
        policy.revalidate_window = Duration::ZERO;

        // Act
        let (before_ttl, at_ttl) = (
            policy.assess(0, fetched(), fetched() + policy.ttl - MS),
            policy.assess(0, fetched(), fetched() + policy.ttl),
        );

        // Assert
        assert_eq!(
            (before_ttl, at_ttl),
            (Freshness::Revalidate, Freshness::Expired)
        );
    }

    #[test]
    fn fetched_in_the_future_is_fresh_with_zero_age() {
        // Arrange
        let policy = policy();
        let now = fetched();
        let from_future = now + Duration::from_secs(60);

        // Act
        let (zone, measured) = (policy.assess(0, from_future, now), age(from_future, now));

        // Assert
        assert_eq!((zone, measured), (Freshness::Fresh, Duration::ZERO));
    }

    #[test]
    fn age_is_elapsed_time() {
        // Arrange
        let now = fetched() + Duration::from_secs(42);

        // Act
        let measured = age(fetched(), now);

        // Assert
        assert_eq!(measured, Duration::from_secs(42));
    }

    #[test]
    fn usable_on_error_ends_exactly_at_ttl_plus_stale_if_error() {
        // Arrange
        let policy = policy();
        let limit = Duration::from_secs(1100);

        // Act
        let (before, at) = (
            policy.usable_on_error(0, fetched(), fetched() + limit - MS),
            policy.usable_on_error(0, fetched(), fetched() + limit),
        );

        // Assert
        assert_eq!((before, at), (true, false));
    }

    #[test]
    fn usable_on_error_uses_adaptive_ttl() {
        // Arrange
        let policy = policy();
        let age = Duration::from_secs(1150); // eff = 200 с при unchanged = 3: предел 1200 с

        // Act
        let (adaptive, base) = (
            policy.usable_on_error(3, fetched(), fetched() + age),
            policy.usable_on_error(0, fetched(), fetched() + age),
        );

        // Assert
        assert_eq!((adaptive, base), (true, false));
    }

    #[test]
    fn keep_for_is_ttl_plus_the_longer_of_window_and_stale_if_error() {
        // Arrange
        let long_stale = policy();
        let mut long_window = policy();
        long_window.revalidate_window = Duration::from_secs(5000);

        // Act
        let (with_stale, with_window, adaptive) = (
            long_stale.keep_for(0),
            long_window.keep_for(0),
            long_stale.keep_for(3),
        );

        // Assert
        assert_eq!(
            (with_stale, with_window, adaptive),
            (
                Duration::from_secs(1100),
                Duration::from_mins(85),
                Duration::from_mins(20)
            )
        );
    }

    #[test]
    fn negative_ttl_doubles_per_miss_and_is_capped() {
        // Arrange
        let policy = NegativePolicy::ARTIFACT;
        let cases = [
            (0, 300),
            (1, 300),
            (2, 600),
            (3, 1200),
            (4, 2400),
            (5, 3600),
            (33, 3600),
            (u32::MAX, 3600),
        ];

        // Act
        let ttls = cases.map(|(misses, _)| policy.ttl_after(misses));

        // Assert
        assert_eq!(ttls, cases.map(|(_, secs)| Duration::from_secs(secs)));
    }

    #[test]
    fn negative_ttl_does_not_overflow_with_huge_base() {
        // Arrange
        let policy = NegativePolicy {
            ttl: Duration::MAX,
            max: Duration::from_secs(10),
        };

        // Act
        let ttl = policy.ttl_after(31);

        // Assert
        assert_eq!(ttl, Duration::from_secs(10));
    }

    #[test]
    fn negative_entry_expires_exactly_at_grown_ttl() {
        // Arrange
        let policy = NegativePolicy::ARTIFACT;
        let limit = Duration::from_secs(600); // два промаха

        // Act
        let (before, at, at_single_miss_limit) = (
            policy.is_expired(2, fetched(), fetched() + limit - MS),
            policy.is_expired(2, fetched(), fetched() + limit),
            policy.is_expired(2, fetched(), fetched() + Duration::from_secs(300)),
        );

        // Assert
        assert_eq!((before, at, at_single_miss_limit), (false, true, false));
    }
}
