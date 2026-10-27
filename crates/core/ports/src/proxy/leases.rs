use std::time::Duration;

use crane_domain::proxy::ProxyKey;

/// Право вести выборку ключа. Не `Clone` (как `Staged`): один lease нельзя отпустить дважды.
#[derive(Debug, PartialEq, Eq)]
pub struct Lease {
    key: ProxyKey,
    token: u64,
}

impl Lease {
    #[must_use]
    pub const fn new(key: ProxyKey, token: u64) -> Self {
        Self { key, token }
    }

    #[must_use]
    pub const fn key(&self) -> &ProxyKey {
        &self.key
    }

    /// Фенсинг-токен: по нему `release` отличает своего держателя от перехватившего.
    #[must_use]
    pub const fn token(&self) -> u64 {
        self.token
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Claim {
    /// Мы ведём выборку и обязаны вызвать `release`.
    Lead(Lease),
    /// Другой лидер закончил (или терпение вышло): перечитать кэш; промах — идти самому.
    Settled,
}

#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    #[error("ошибка координатора выборок")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Межрепликовый singleflight. В процессе выборки схлопывает `app`; этот порт нужен только
/// между репликами (Valkey: `SET NX PX` и канал `inv`).
pub trait FetchLeases: Send + Sync {
    /// Стать лидером по ключу на `lease` или дождаться чужого лидера не дольше `patience`.
    fn claim(
        &self,
        key: &ProxyKey,
        lease: Duration,
        patience: Duration,
    ) -> impl Future<Output = Result<Claim, LeaseError>> + Send;

    /// Снимает lease, только если токен совпадает с текущим держателем (фенсинг), и будит ждущих.
    /// Чужой или истёкший-и-перехваченный lease — не ошибка, а no-op.
    ///
    /// Снятие начинается уже при вызове: возвращённый future можно не опрашивать (путь отмены — `Drop` —
    /// не умеет `await`); опрос лишь подтверждает завершение. Адаптер с сетью (Valkey, п. 2) инициирует
    /// снятие синхронно и не выпускает следующий `claim` того же ключа в этом процессе раньше него.
    fn release(&self, lease: Lease) -> impl Future<Output = Result<(), LeaseError>> + Send;
}
