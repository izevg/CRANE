//! `MetadataPolicy::usable_when_throttled`: окно stale при ограничении запросов.

use std::num::NonZeroU32;
use std::time::{Duration, SystemTime};

use crane_domain::proxy::{MetadataPolicy, NegativePolicy};

const MS: Duration = Duration::from_millis(1);
const DAY: Duration = Duration::from_hours(24);

fn policy(stale_if_error: Duration) -> MetadataPolicy {
    MetadataPolicy {
        ttl: Duration::from_secs(100),
        revalidate_window: Duration::from_secs(100),
        stale_if_error,
        adaptive_after: NonZeroU32::MIN,
        negative: NegativePolicy::ARTIFACT,
    }
}

fn fetched() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000)
}

#[test]
fn usable_when_throttled_is_capped_at_a_day_past_ttl() {
    // Arrange
    let policy = policy(Duration::from_hours(168));
    let limit = policy.ttl + DAY;

    // Act
    let (before, at) = (
        policy.usable_when_throttled(0, None, fetched(), fetched() + limit - MS),
        policy.usable_when_throttled(0, None, fetched(), fetched() + limit),
    );

    // Assert
    assert_eq!((before, at), (true, false));
}

#[test]
fn usable_when_throttled_keeps_shorter_stale_if_error_window() {
    // Arrange
    let policy = policy(Duration::from_secs(1000));
    let limit = Duration::from_secs(1100);

    // Act
    let (before, at) = (
        policy.usable_when_throttled(0, None, fetched(), fetched() + limit - MS),
        policy.usable_when_throttled(0, None, fetched(), fetched() + limit),
    );

    // Assert
    assert_eq!((before, at), (true, false));
}
