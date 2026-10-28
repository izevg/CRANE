//! Перевод `SystemTime` <-> `timestamptz` без паник на крайних значениях. `None` значит «вне
//! диапазона»: тип ошибки выбирает вызывающий (каталог, настройки).

use std::time::{Duration, SystemTime};

use time::OffsetDateTime;

pub(crate) fn to_db_time(time: SystemTime) -> Option<OffsetDateTime> {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => time::Duration::try_from(after)
            .ok()
            .and_then(|d| OffsetDateTime::UNIX_EPOCH.checked_add(d)),
        Err(before) => time::Duration::try_from(before.duration())
            .ok()
            .and_then(|d| OffsetDateTime::UNIX_EPOCH.checked_sub(d)),
    }
}

pub(crate) fn from_db_time(time: OffsetDateTime) -> Option<SystemTime> {
    let delta = time - OffsetDateTime::UNIX_EPOCH;
    let magnitude: Duration = delta.unsigned_abs();
    if delta.is_negative() {
        SystemTime::UNIX_EPOCH.checked_sub(magnitude)
    } else {
        SystemTime::UNIX_EPOCH.checked_add(magnitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_time_round_trips_before_and_after_epoch() {
        // Arrange
        let moments = [
            SystemTime::UNIX_EPOCH - Duration::from_hours(24),
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH + Duration::from_micros(1_700_000_000_123_456),
        ];

        // Act
        let back = moments.map(|m| to_db_time(m).and_then(from_db_time));

        // Assert
        assert_eq!(back, moments.map(Some));
    }
}
