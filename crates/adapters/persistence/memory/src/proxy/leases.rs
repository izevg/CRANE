use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime};

use crane_domain::proxy::ProxyKey;
use crane_ports::proxy::{Claim, FetchLeases, Lease, LeaseError};
use crane_ports::system::Clock;
use futures_channel::oneshot;

/// Координатор выборок в памяти: настоящая lease-семантика (срок, фенсинг-токен, пробуждение
/// ждущих). Эталон контракта и тестов. В продакшен не годится: без рантайма нет `patience`,
/// и ждущий брошенного лидера (отменённый клиент) висит до перехвата lease следующим `claim`
/// после истечения, а если запросов больше нет, то без срока.
///
/// В single-node межрепликовый координатор не нужен (полёты схлопывает `app`); продакшен-сборку
/// решает `entry`. Таймаут ожидания — обязанность Valkey-адаптера.
pub struct MemoryFetchLeases {
    clock: Arc<dyn Clock>,
    next_token: AtomicU64,
    holders: Mutex<HashMap<ProxyKey, Holder>>,
}

struct Holder {
    token: u64,
    /// `None` — lease не истекает (срок не представим в `SystemTime`).
    expires_at: Option<SystemTime>,
    waiters: Vec<oneshot::Sender<()>>,
}

impl fmt::Debug for MemoryFetchLeases {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `dyn Clock` не Debug.
        f.debug_struct("MemoryFetchLeases").finish_non_exhaustive()
    }
}

impl MemoryFetchLeases {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            next_token: AtomicU64::new(0),
            holders: Mutex::new(HashMap::new()),
        }
    }

    // Замок берётся только в синхронных функциях: guard не должен жить через await.
    fn lock(&self) -> MutexGuard<'_, HashMap<ProxyKey, Holder>> {
        self.holders.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Либо выдаёт lease, либо ставит в очередь к текущему держателю.
    fn try_claim(&self, key: &ProxyKey, lease: Duration) -> Result<Lease, oneshot::Receiver<()>> {
        let now = self.clock.now();
        let mut holders = self.lock();
        if let Some(holder) = holders.get_mut(key) {
            // Граница включительна: на `expires_at` lease уже не действует.
            let expired = holder.expires_at.is_some_and(|at| at <= now);
            if !expired {
                let (tx, rx) = oneshot::channel();
                holder.waiters.push(tx);
                return Err(rx);
            }
        }
        // Токены с 1: 0 остаётся «никем не выданным».
        let token = self.next_token.fetch_add(1, Ordering::Relaxed) + 1;
        let previous = holders.insert(
            key.clone(),
            Holder {
                token,
                expires_at: now.checked_add(lease),
                waiters: Vec::new(),
            },
        );
        // Лидер умер, не сняв lease: его ждущие перечитают кэш и, при промахе, пойдут сами.
        for waiter in previous.into_iter().flat_map(|holder| holder.waiters) {
            let _ = waiter.send(());
        }
        Ok(Lease::new(key.clone(), token))
    }

    fn drop_lease(&self, lease: &Lease) {
        let mut holders = self.lock();
        if holders
            .get(lease.key())
            .is_some_and(|holder| holder.token == lease.token())
            && let Some(holder) = holders.remove(lease.key())
        {
            for waiter in holder.waiters {
                // Ждущий мог уйти: это не ошибка.
                let _ = waiter.send(());
            }
        }
    }
}

impl FetchLeases for MemoryFetchLeases {
    async fn claim(
        &self,
        key: &ProxyKey,
        lease: Duration,
        _patience: Duration,
    ) -> Result<Claim, LeaseError> {
        match self.try_claim(key, lease) {
            Ok(lease) => Ok(Claim::Lead(lease)),
            // Любой исход, в том числе отброшенный sender, значит «держатель ушёл».
            Err(waiting) => {
                let _ = waiting.await;
                Ok(Claim::Settled)
            }
        }
    }

    fn release(&self, lease: Lease) -> impl Future<Output = Result<(), LeaseError>> + Send {
        self.drop_lease(&lease);
        std::future::ready(Ok(()))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "тесты: падение = провал проверки"
)]
mod tests {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use crane_domain::proxy::UpstreamPath;
    use crane_domain::registry::RepositoryId;
    use uuid::Uuid;

    use super::*;
    use crate::ManualClock;

    const LEASE: Duration = Duration::from_secs(10);
    const PATIENCE: Duration = Duration::from_secs(1);

    fn key() -> ProxyKey {
        ProxyKey {
            repository: RepositoryId::new(Uuid::from_u128(1)),
            path: UpstreamPath::parse("a/b").unwrap(),
        }
    }

    fn setup() -> (Arc<ManualClock>, MemoryFetchLeases) {
        let clock = Arc::new(ManualClock::new(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
        ));
        let leases = MemoryFetchLeases::new(clock.clone());
        (clock, leases)
    }

    fn lead(claim: Claim) -> Lease {
        match claim {
            Claim::Lead(lease) => lease,
            Claim::Settled => panic!("ожидался Lead"),
        }
    }

    macro_rules! poll_once {
        ($future:expr) => {
            $future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
        };
    }

    #[test]
    fn expired_lease_is_taken_over_with_new_token() {
        // Arrange
        let (clock, leases) = setup();
        let dead = lead(futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap());
        clock.advance(LEASE);

        // Act
        let result = futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap();

        // Assert
        let taker = lead(result);
        assert_ne!(taker.token(), dead.token(), "перехват выдаёт новый токен");
    }

    #[test]
    fn claim_one_millisecond_before_expiry_still_waits() {
        // Arrange
        let (clock, leases) = setup();
        let _holder =
            lead(futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap());
        clock.advance(LEASE.checked_sub(Duration::from_millis(1)).unwrap());
        let probe_key = key();
        let mut second = pin!(leases.claim(&probe_key, LEASE, PATIENCE));

        // Act
        let polled = poll_once!(second);

        // Assert
        assert!(polled.is_pending(), "lease ещё действует: претендент ждёт");
    }

    #[test]
    fn claim_exactly_at_expiry_takes_over() {
        // Arrange
        let (clock, leases) = setup();
        let _holder =
            lead(futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap());
        clock.advance(LEASE);
        let probe_key = key();
        let mut second = pin!(leases.claim(&probe_key, LEASE, PATIENCE));

        // Act
        let polled = poll_once!(second);

        // Assert
        assert!(
            matches!(polled, Poll::Ready(Ok(Claim::Lead(_)))),
            "на границе expires_at lease уже не действует"
        );
    }

    #[test]
    fn release_of_taken_over_lease_keeps_new_holder() {
        // Arrange
        let (clock, leases) = setup();
        let dead = lead(futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap());
        clock.advance(LEASE);
        let _taker =
            lead(futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap());
        futures_executor::block_on(leases.release(dead)).unwrap();
        let probe_key = key();
        let mut third = pin!(leases.claim(&probe_key, LEASE, PATIENCE));

        // Act
        let polled = poll_once!(third);

        // Assert
        assert!(
            polled.is_pending(),
            "release устаревшего токена не снимает нового держателя"
        );
    }

    #[test]
    fn takeover_of_expired_lease_settles_its_waiters() {
        // Arrange
        let (clock, leases) = setup();
        let _dead =
            lead(futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap());
        let waiting_key = key();
        let mut waiter = pin!(leases.claim(&waiting_key, LEASE, PATIENCE));
        let queued = poll_once!(waiter).is_pending();
        clock.advance(LEASE);

        // Act
        let _taker =
            lead(futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap());

        // Assert
        assert!(queued, "ждущий встал в очередь");
        assert!(
            matches!(poll_once!(waiter), Poll::Ready(Ok(Claim::Settled))),
            "ждущие умершего лидера получают Settled при перехвате"
        );
    }

    #[test]
    fn release_wakes_waiter_with_settled() {
        // Arrange
        let (_clock, leases) = setup();
        let holder =
            lead(futures_executor::block_on(leases.claim(&key(), LEASE, PATIENCE)).unwrap());
        let waiting_key = key();
        let mut waiter = pin!(leases.claim(&waiting_key, LEASE, PATIENCE));
        let queued = poll_once!(waiter).is_pending();

        // Act
        futures_executor::block_on(leases.release(holder)).unwrap();

        // Assert
        assert!(queued, "ждущий встал в очередь");
        assert!(
            matches!(poll_once!(waiter), Poll::Ready(Ok(Claim::Settled))),
            "release будит ждущего сразу"
        );
    }
}
