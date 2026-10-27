#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

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
    let ttls = cases.map(|(unchanged, _)| policy.effective_ttl(unchanged, None));

    // Assert
    assert_eq!(ttls, cases.map(|(_, factor)| policy.ttl * factor));
}

#[test]
fn effective_ttl_does_not_overflow_on_huge_base() {
    // Arrange
    let mut policy = policy();
    policy.ttl = Duration::MAX;

    // Act
    let ttl = policy.effective_ttl(100, None);

    // Assert
    assert_eq!(ttl, Duration::MAX);
}

#[test]
fn assess_switches_zones_exactly_on_boundaries() {
    // Arrange
    let policy = policy();
    let fresh_until = Duration::from_secs(90);
    let stale_from = Duration::from_secs(100);
    let expired_from = Duration::from_secs(200);
    let cases = [
        (Duration::ZERO, Freshness::Fresh),
        (ms_before(fresh_until), Freshness::Fresh),
        (fresh_until, Freshness::RefreshAhead),
        (ms_before(stale_from), Freshness::RefreshAhead),
        (stale_from, Freshness::Stale),
        (ms_before(expired_from), Freshness::Stale),
        (expired_from, Freshness::Expired),
        (Duration::from_secs(100_000), Freshness::Expired),
    ];

    // Act
    let zones = cases.map(|(age, _)| policy.assess(0, None, fetched(), fetched() + age));

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
        (Duration::from_secs(180), Freshness::RefreshAhead),
        (Duration::from_secs(199), Freshness::RefreshAhead),
        (Duration::from_secs(200), Freshness::Stale),
        (Duration::from_secs(299), Freshness::Stale),
        (Duration::from_secs(300), Freshness::Expired),
    ];

    // Act
    let zones = cases.map(|(age, _)| policy.assess(unchanged, None, fetched(), fetched() + age));

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
        policy.assess(0, None, fetched(), fetched() + policy.ttl - MS),
        policy.assess(0, None, fetched(), fetched() + policy.ttl),
    );

    // Assert
    assert_eq!(
        (before_ttl, at_ttl),
        (Freshness::RefreshAhead, Freshness::Expired)
    );
}

#[test]
fn fetched_in_the_future_is_fresh_with_zero_age() {
    // Arrange
    let policy = policy();
    let now = fetched();
    let from_future = now + Duration::from_secs(60);

    // Act
    let (zone, measured) = (
        policy.assess(0, None, from_future, now),
        age(from_future, now),
    );

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
        policy.usable_on_error(0, None, fetched(), fetched() + limit - MS),
        policy.usable_on_error(0, None, fetched(), fetched() + limit),
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
        policy.usable_on_error(3, None, fetched(), fetched() + age),
        policy.usable_on_error(0, None, fetched(), fetched() + age),
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
        long_stale.keep_for(0, None),
        long_window.keep_for(0, None),
        long_stale.keep_for(3, None),
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
fn effective_ttl_lets_upstream_hint_extend_but_not_shorten_or_exceed_ceiling() {
    // Arrange
    let policy = policy(); // ttl 100 с, потолок 800 с
    let secs = Duration::from_secs;
    let cases = [
        (0, None, 100),
        (0, Some(secs(50)), 100),
        (0, Some(secs(300)), 300),
        (0, Some(secs(800)), 800),
        (0, Some(secs(5000)), 800),
        (3, Some(secs(150)), 200),
    ];

    // Act
    let ttls = cases.map(|(unchanged, hint, _)| policy.effective_ttl(unchanged, hint));

    // Assert
    assert_eq!(ttls, cases.map(|(_, _, expected)| secs(expected)));
}

#[test]
fn assess_counts_upstream_hint_toward_freshness() {
    // Arrange
    let mut policy = policy();
    policy.ttl = Duration::from_mins(5);
    let age = Duration::from_mins(7);

    // Act
    let (with_hint, without_hint) = (
        policy.assess(0, Some(Duration::from_mins(10)), fetched(), fetched() + age),
        policy.assess(0, None, fetched(), fetched() + age),
    );

    // Assert
    assert_eq!(
        (with_hint, without_hint),
        (Freshness::Fresh, Freshness::Expired)
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
