use std::num::NonZeroU32;
use std::time::{Duration, SystemTime};

/// Потолок множителя адаптивного TTL: 2^3 = ×8 (docs/product/11-heuristics.md, 11.1).
const ADAPTIVE_MAX_DOUBLINGS: u32 = 3;

/// Потолок stale при ограничении запросов (429/403). Rate limit атакующий может спровоцировать,
/// и без потолка stale-if-error (до недели) заморозил бы документы; сутки же переживают
/// короткие блокировки, не отказывая в доступности.
const STALE_IF_THROTTLED_MAX: Duration = Duration::from_hours(24);

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
    /// Последние 10% TTL: отдать как свежее и обновить в фоне (refresh-ahead).
    RefreshAhead,
    /// После TTL, но в окне: отдать как устаревшее и обновить в фоне (stale-while-revalidate).
    Stale,
    /// Идти в upstream блокирующим запросом.
    Expired,
}

impl MetadataPolicy {
    /// Срок свежести: адаптивный TTL `ttl · 2^min(unchanged / adaptive_after, 3)` (потолок ×8, 11.1),
    /// который подсказка upstream `max_age` может продлить, но не сократить и не поднять выше потолка.
    #[must_use]
    pub fn effective_ttl(&self, unchanged: u32, max_age: Option<Duration>) -> Duration {
        let doublings = (unchanged / self.adaptive_after.get()).min(ADAPTIVE_MAX_DOUBLINGS);
        let adaptive = self.ttl.saturating_mul(1 << doublings);
        let ceiling = self.ttl.saturating_mul(1 << ADAPTIVE_MAX_DOUBLINGS);
        // Подсказка upstream продлевает срок, но не сокращает его и не выходит за адаптивный потолок (11.1).
        max_age.map_or(adaptive, |hint| adaptive.max(hint.min(ceiling)))
    }

    /// Зоны по возрасту записи. Границы целочисленные (без f64): `eff − eff/10` (последние 10% TTL) и `eff`.
    #[must_use]
    pub fn assess(
        &self,
        unchanged: u32,
        max_age: Option<Duration>,
        fetched_at: SystemTime,
        now: SystemTime,
    ) -> Freshness {
        let effective = self.effective_ttl(unchanged, max_age);
        let age = age(fetched_at, now);
        if age < effective.saturating_sub(effective / 10) {
            Freshness::Fresh
        } else if age < effective {
            Freshness::RefreshAhead
        } else if age < effective.saturating_add(self.revalidate_window) {
            Freshness::Stale
        } else {
            Freshness::Expired
        }
    }

    /// Запись ещё годится при сбое upstream.
    #[must_use]
    pub fn usable_on_error(
        &self,
        unchanged: u32,
        max_age: Option<Duration>,
        fetched_at: SystemTime,
        now: SystemTime,
    ) -> bool {
        self.usable_within(self.stale_if_error, unchanged, max_age, fetched_at, now)
    }

    /// Запись ещё годится, когда upstream ограничил запросы: окно stale-if-error, но не дольше
    /// `STALE_IF_THROTTLED_MAX`.
    #[must_use]
    pub fn usable_when_throttled(
        &self,
        unchanged: u32,
        max_age: Option<Duration>,
        fetched_at: SystemTime,
        now: SystemTime,
    ) -> bool {
        let window = self.stale_if_error.min(STALE_IF_THROTTLED_MAX);
        self.usable_within(window, unchanged, max_age, fetched_at, now)
    }

    fn usable_within(
        &self,
        window: Duration,
        unchanged: u32,
        max_age: Option<Duration>,
        fetched_at: SystemTime,
        now: SystemTime,
    ) -> bool {
        age(fetched_at, now)
            < self
                .effective_ttl(unchanged, max_age)
                .saturating_add(window)
    }

    /// Сколько хранить запись (TTL ключа L2): пока она может понадобиться как stale или как резерв при сбое.
    #[must_use]
    pub fn keep_for(&self, unchanged: u32, max_age: Option<Duration>) -> Duration {
        self.effective_ttl(unchanged, max_age)
            .saturating_add(self.revalidate_window.max(self.stale_if_error))
    }
}

#[cfg(test)]
#[path = "tests/policy.rs"]
mod tests;
