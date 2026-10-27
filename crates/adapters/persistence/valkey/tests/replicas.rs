//! Сценарии «две реплики»: два экземпляра `Valkey` на одной БД, как два процесса CRANE.

#![allow(clippy::expect_used, reason = "тесты: падение = провал проверки")]

#[allow(
    dead_code,
    reason = "помощники общие для нескольких тестовых бинарников"
)]
mod support;

use std::pin::pin;
use std::task::Context;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_domain::proxy::{ProxyKey, UpstreamPath};
use crane_domain::registry::RepositoryId;
use crane_ports::proxy::{Cached, CachedState, Claim, FetchLeases, ProxyCache};
use crane_valkey::Valkey;
use futures_util::task::noop_waker_ref;
use uuid::Uuid;

// Каждому тесту своя БД Valkey: функции идут параллельно, а `FLUSHDB` очищает всю БД.
const INVALIDATION_DB: u8 = 3;
const WAKE_DB: u8 = 4;
const FENCING_DB: u8 = 5;
const PATIENCE_DB: u8 = 6;

const KEEP: Duration = Duration::from_secs(60);
const LEASE: Duration = Duration::from_secs(30);
const SHORT_LEASE: Duration = Duration::from_millis(100);
/// С запасом больше `SHORT_LEASE`: сервер обязан успеть истечь ключ.
const AFTER_SHORT_LEASE: Duration = Duration::from_millis(250);
const LONG_PATIENCE: Duration = Duration::from_secs(30);
const SHORT_PATIENCE: Duration = Duration::from_millis(200);
/// Предел ожидания в тесте пробуждения: заведомо меньше `LONG_PATIENCE`.
const WAKE_DEADLINE: Duration = Duration::from_secs(10);
/// Предел доставки инвалидации между репликами.
const DELIVERY_DEADLINE: Duration = Duration::from_secs(1);
const POLL_STEP: Duration = Duration::from_millis(10);

fn key() -> ProxyKey {
    ProxyKey {
        repository: RepositoryId::new(Uuid::from_u128(1)),
        path: UpstreamPath::parse("index/se/rd/serde").expect("путь в тестовых данных валиден"),
    }
}

fn document(text: &'static [u8]) -> Cached {
    Cached {
        fetched_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_000),
        state: CachedState::Document {
            body: Bytes::from_static(text),
            etag: None,
            unchanged: 0,
            max_age: None,
        },
    }
}

/// Читает запись, пока она не станет равна `wanted`, но не дольше `DELIVERY_DEADLINE`:
/// pub/sub доставляется асинхронно, и момент прихода инвалидации тест не знает. Единственный
/// опрос с ограничением по времени во всём наборе; по исходу сравнивает вызывающий тест.
async fn get_until(cache: &Valkey, wanted: &Cached) -> Option<Cached> {
    let deadline = tokio::time::Instant::now() + DELIVERY_DEADLINE;
    loop {
        let found = cache.get(&key()).await.expect("get");
        if found.as_ref() == Some(wanted) || tokio::time::Instant::now() >= deadline {
            return found;
        }
        tokio::time::sleep(POLL_STEP).await;
    }
}

fn lead(claim: Claim) -> crane_ports::proxy::Lease {
    match claim {
        Claim::Lead(lease) => lease,
        Claim::Settled => unreachable!("ключ свободен: ожидался Lead"),
    }
}

#[test]
fn put_on_one_replica_drops_stale_entry_from_l1_of_another() {
    support::run(async |server| {
        // Arrange
        let (a, b) = server.replicas(INVALIDATION_DB).await;
        let (stale, fresh) = (document(b"v0"), document(b"v1"));
        a.put(&key(), stale.clone(), KEEP).await.expect("put v0");
        let warmed = b.get(&key()).await.expect("get v0");

        // Act
        let put = a.put(&key(), fresh.clone(), KEEP).await;

        // Assert
        put.expect("put v1");
        assert_eq!(warmed, Some(stale), "вторая реплика закэшировала v0 в L1");
        assert_eq!(
            get_until(&b, &fresh).await,
            Some(fresh),
            "запись первой реплики выбрасывает устаревшее из L1 второй"
        );
    });
}

#[test]
fn release_on_one_replica_wakes_waiting_claim_on_another() {
    support::run(async |server| {
        // Arrange
        let (a, b) = server.replicas(WAKE_DB).await;
        let leader = lead(
            a.claim(&key(), LEASE, LONG_PATIENCE)
                .await
                .expect("claim A"),
        );
        let waiting_key = key();
        let mut waiting = pin!(b.claim(&waiting_key, LEASE, LONG_PATIENCE));
        let queued = waiting
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending();
        // Команды одного соединения упорядочены: когда ответит GET, SET ждущего уже обработан,
        // и release первой реплики не может обогнать его.
        b.get(&waiting_key).await.expect("барьер соединения");

        // Act
        let released = a.release(leader).await;

        // Assert
        released.expect("release");
        assert!(queued, "пока лидер держит ключ, claim второй реплики ждёт");
        assert_eq!(
            tokio::time::timeout(WAKE_DEADLINE, waiting)
                .await
                .ok()
                .and_then(Result::ok),
            Some(Claim::Settled),
            "release будит ждущего на другой реплике задолго до patience"
        );
    });
}

#[test]
fn release_of_expired_token_keeps_new_holder() {
    support::run(async |server| {
        // Arrange
        let (a, b) = server.replicas(FENCING_DB).await;
        let expired = lead(
            a.claim(&key(), SHORT_LEASE, LONG_PATIENCE)
                .await
                .expect("claim A"),
        );
        tokio::time::sleep(AFTER_SHORT_LEASE).await;
        let _holder = lead(
            b.claim(&key(), LEASE, LONG_PATIENCE)
                .await
                .expect("claim B"),
        );

        // Act
        let released = a.release(expired).await;

        // Assert
        released.expect("release устаревшего токена - no-op, а не ошибка");
        assert_eq!(
            b.claim(&key(), LEASE, SHORT_PATIENCE).await.expect("claim"),
            Claim::Settled,
            "новый держатель сохранил lease: третий претендент ждёт, а не лидирует"
        );
    });
}

#[test]
fn claim_settles_when_patience_runs_out() {
    support::run(async |server| {
        // Arrange
        let (a, b) = server.replicas(PATIENCE_DB).await;
        let _holder = lead(
            a.claim(&key(), LEASE, LONG_PATIENCE)
                .await
                .expect("claim A"),
        );

        // Act
        let result = b.claim(&key(), LEASE, SHORT_PATIENCE).await;

        // Assert
        assert_eq!(
            result.expect("claim B"),
            Claim::Settled,
            "держатель не отпустил: после patience претендент перечитывает кэш"
        );
    });
}
