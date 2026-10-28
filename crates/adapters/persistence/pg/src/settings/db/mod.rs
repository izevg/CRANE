//! Общее для хранилищ настроек и мастера: ошибки драйвера и время.

use std::error::Error;
use std::io;
use std::time::{Duration, SystemTime};

use time::OffsetDateTime;

/// Ошибка адаптера в форме, которую несут `Backend`-варианты портов.
pub(super) type BoxError = Box<dyn Error + Send + Sync>;

/// Сбой драйвера или БД. Текст ошибки `sqlx` не содержит ни значений, ни строки подключения.
pub(super) fn driver(error: sqlx::Error) -> BoxError {
    Box::new(error)
}

/// Строка или значение не укладываются в модель: данные повреждены или схема разошлась с кодом.
pub(super) fn inconsistent(what: &str) -> BoxError {
    Box::new(io::Error::other(what.to_owned()))
}

/// `SystemTime` -> `timestamptz` без паник на крайних значениях.
pub(super) fn to_db_time(time: SystemTime) -> Result<OffsetDateTime, BoxError> {
    let shifted = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => time::Duration::try_from(after)
            .ok()
            .and_then(|d| OffsetDateTime::UNIX_EPOCH.checked_add(d)),
        Err(before) => time::Duration::try_from(before.duration())
            .ok()
            .and_then(|d| OffsetDateTime::UNIX_EPOCH.checked_sub(d)),
    };
    shifted.ok_or_else(|| inconsistent("момент времени вне диапазона timestamptz"))
}

pub(super) fn from_db_time(time: OffsetDateTime) -> Result<SystemTime, BoxError> {
    let delta = time - OffsetDateTime::UNIX_EPOCH;
    let magnitude: Duration = delta.unsigned_abs();
    let shifted = if delta.is_negative() {
        SystemTime::UNIX_EPOCH.checked_sub(magnitude)
    } else {
        SystemTime::UNIX_EPOCH.checked_add(magnitude)
    };
    shifted.ok_or_else(|| inconsistent("момент времени вне диапазона SystemTime"))
}
