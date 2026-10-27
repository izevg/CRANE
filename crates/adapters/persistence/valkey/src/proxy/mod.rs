mod cache;
mod codec;
mod leases;

pub(crate) use leases::Releases;

use std::time::Duration;

use crane_domain::proxy::ProxyKey;
use fred::types::Expiration;

const MILLIS_PER_SECOND: u64 = 1_000;
const NANOS_PER_MILLI: u32 = 1_000_000;

/// Верхняя граница TTL в мс. Сервер отклоняет `PX`, если `now + ttl` не помещается в i64,
/// поэтому предел взят с запасом в половину диапазона (около 146 млн лет): всё, что больше,
/// для кэша равнозначно записи без срока.
const MAX_EXPIRY_MS: i64 = i64::MAX / 2;

/// Имя записи кэша. Без фигурных скобок: в Cluster они задают hash tag. UUID фиксированной
/// длины, поэтому `:` в пути не даёт неоднозначности.
pub(crate) fn entry_key(key: &ProxyKey) -> String {
    format!(
        "px:{}:{}",
        key.repository.as_uuid().hyphenated(),
        key.path.as_str()
    )
}

/// Имя lease для записи; устроено так же, как [`entry_key`].
pub(crate) fn lease_key(key: &ProxyKey) -> String {
    format!(
        "sf:{}:{}",
        key.repository.as_uuid().hyphenated(),
        key.path.as_str()
    )
}

/// TTL ключа в миллисекундах с округлением вверх и минимумом 1 (`PX 0` - ошибка Valkey).
/// `None` - срок не представим (в том числе `Duration::MAX`): ключ пишется без TTL.
fn expiry(keep_for: Duration) -> Option<i64> {
    let whole = keep_for.as_secs().checked_mul(MILLIS_PER_SECOND)?;
    let fraction = u64::from(keep_for.subsec_nanos().div_ceil(NANOS_PER_MILLI));
    let millis = whole.checked_add(fraction)?.max(1);
    i64::try_from(millis)
        .ok()
        .filter(|millis| *millis <= MAX_EXPIRY_MS)
}

/// Аргумент `PX` для SET.
fn expiration(keep_for: Duration) -> Option<Expiration> {
    expiry(keep_for).map(Expiration::PX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_rounds_up_to_whole_milliseconds_with_floor_of_one() {
        // Arrange
        let cases = [
            Duration::ZERO,
            Duration::from_nanos(1),
            Duration::from_millis(1),
            Duration::from_micros(1_500),
            Duration::from_secs(60),
        ];

        // Act
        let actual = cases.map(expiry);

        // Assert
        assert_eq!(actual, [Some(1), Some(1), Some(1), Some(2), Some(60_000)]);
    }

    #[test]
    fn expiry_drops_ttl_that_px_cannot_carry() {
        // Arrange
        let limit = Duration::from_millis(MAX_EXPIRY_MS.unsigned_abs());
        let cases = [
            limit,
            limit + Duration::from_millis(1),
            Duration::from_secs(u64::MAX),
            Duration::MAX,
        ];

        // Act
        let actual = cases.map(expiry);

        // Assert
        assert_eq!(actual, [Some(MAX_EXPIRY_MS), None, None, None]);
    }
}
