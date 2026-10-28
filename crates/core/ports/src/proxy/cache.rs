use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_domain::proxy::ProxyKey;
use crane_domain::registry::{Checksum, Digests, Sha1, Sha256, Sha512};

/// Непрозрачный валидатор upstream для условных запросов (`If-None-Match`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ETag(String);

impl ETag {
    #[must_use]
    pub const fn new(raw: String) -> Self {
        Self(raw)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Запись proxy-кэша: только факты (когда получена, что получено, сколько раз подряд не менялась).
///
/// Сроки не запечены: их выводит политика domain при чтении, поэтому смена конфигурации
/// действует на лежащие записи.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cached {
    pub fetched_at: SystemTime,
    pub state: CachedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedState {
    /// Сырое тело metadata upstream; переписывание ссылок — дело формата при отдаче.
    /// `unchanged` — сколько обновлений подряд тело не менялось (адаптивный TTL).
    Document {
        body: Bytes,
        etag: Option<ETag>,
        unchanged: u32,
        /// Срок свежести, объявленный upstream (`Cache-Control`), до обрезки границами репозитория;
        /// `None` — upstream срока не объявил, действует TTL политики. Целые секунды: так его
        /// передаёт заголовок, и бэкенды обязаны хранить его без потерь до секунды.
        max_age: Option<Duration>,
    },
    /// Артефакт лежит в CAS; запись хранит только дайджесты. Неизменяем: `ETag` и счётчик не нужны.
    Blob(CachedDigests),
    /// Upstream ответил 404. `misses` — сколько раз подряд (рост negative TTL).
    Missing { misses: u32 },
}

/// Дайджесты проксированного артефакта. Уже, чем `Digests`: MD5 в запись кэша не входит
/// (формат записи Valkey его не хранит), а клиентам proxy MD5 не отдаёт.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedDigests {
    pub sha256: Sha256,
    pub sha512: Sha512,
    pub sha1: Sha1,
    pub size: u64,
}

impl From<&Digests> for CachedDigests {
    fn from(digests: &Digests) -> Self {
        Self {
            sha256: digests.sha256,
            sha512: digests.sha512,
            sha1: digests.sha1,
            size: digests.size,
        }
    }
}

impl CachedDigests {
    /// Совпадает ли ожидаемая сумма с посчитанной тем же алгоритмом. MD5 не хранится,
    /// поэтому ожидание MD5 не выполнимо никогда.
    #[must_use]
    pub fn satisfies(&self, expected: &Checksum) -> bool {
        !matches!(expected, Checksum::Md5(_)) && self.of_kind(expected) == *expected
    }

    /// Посчитанная сумма того же алгоритма, что и `like`. Для MD5 суммы нет:
    /// возвращается SHA-256, который заведомо не равен ожиданию.
    #[must_use]
    pub const fn of_kind(&self, like: &Checksum) -> Checksum {
        match like {
            Checksum::Sha512(_) => Checksum::Sha512(self.sha512),
            Checksum::Sha1(_) => Checksum::Sha1(self.sha1),
            Checksum::Sha256(_) | Checksum::Md5(_) => Checksum::Sha256(self.sha256),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("ошибка кэша proxy")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Кэш записей proxy. Не источник истины: use case трактует ошибку `get` как промах,
/// ошибку `put` игнорирует.
///
/// Реализации: Valkey-адаптер держит L1 (moka) перед L2 и сбрасывает её по каналу `inv`;
/// single-node — ограниченный кэш в памяти (crane-memory).
pub trait ProxyCache: Send + Sync {
    fn get(
        &self,
        key: &ProxyKey,
    ) -> impl Future<Output = Result<Option<Cached>, CacheError>> + Send;

    /// `keep_for` — сколько записи имеет смысл жить в бэкенде (TTL ключа Valkey); считает use case
    /// из политики. Вытеснение — право бэкенда, а не обязанность: свежесть решает use case.
    /// `Duration::MAX` — запись без срока (неизменяемый артефакт): бэкенд хранит её без TTL.
    fn put(
        &self,
        key: &ProxyKey,
        entry: Cached,
        keep_for: Duration,
    ) -> impl Future<Output = Result<(), CacheError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CachedDigests {
        CachedDigests {
            sha256: Sha256::from([1; 32]),
            sha512: Sha512::from([2; 64]),
            sha1: Sha1::from([3; 20]),
            size: 5,
        }
    }

    #[test]
    fn satisfies_matches_stored_algorithms_and_never_md5() {
        // Arrange
        let digests = sample();
        let cases = [
            Checksum::Sha256(digests.sha256),
            Checksum::Sha512(digests.sha512),
            Checksum::Sha1(digests.sha1),
            Checksum::Sha256(Sha256::from([0; 32])),
            Checksum::Sha512(Sha512::from([0; 64])),
            Checksum::Sha1(Sha1::from([0; 20])),
            Checksum::Md5(crane_domain::registry::Md5::from([0; 16])),
        ];

        // Act
        let actual = cases.map(|c| digests.satisfies(&c));

        // Assert
        assert_eq!(actual, [true, true, true, false, false, false, false]);
    }

    #[test]
    fn of_kind_returns_own_digest_of_requested_algorithm() {
        // Arrange
        let digests = sample();
        let cases = [
            Checksum::Sha256(Sha256::from([0; 32])),
            Checksum::Sha512(Sha512::from([0; 64])),
            Checksum::Sha1(Sha1::from([0; 20])),
        ];

        // Act
        let actual = cases.map(|c| digests.of_kind(&c));

        // Assert
        assert_eq!(
            actual,
            [
                Checksum::Sha256(digests.sha256),
                Checksum::Sha512(digests.sha512),
                Checksum::Sha1(digests.sha1),
            ]
        );
    }
}
