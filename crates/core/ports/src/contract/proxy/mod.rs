use std::pin::pin;
use std::task::Context;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_domain::proxy::{ProxyKey, UpstreamPath};
use crane_domain::registry::{RepositoryId, Sha1, Sha256, Sha512};
use futures_util::task::noop_waker_ref;
use uuid::Uuid;

use crate::proxy::{Cached, CachedState, Claim, ETag, FetchLeases, ProxyCache};
use crate::registry::Digests;

const LEASE: Duration = Duration::from_secs(30);
const PATIENCE: Duration = Duration::from_secs(5);

fn key_of(repository: u128, path: &str) -> ProxyKey {
    ProxyKey {
        repository: RepositoryId::new(Uuid::from_u128(repository)),
        path: UpstreamPath::parse(path).expect("путь в тестовых данных валиден"),
        accept: None,
    }
}

const JSON: &str = "application/vnd.pypi.simple.v1+json";

fn with_accept(mut key: ProxyKey, accept: &'static str) -> ProxyKey {
    key.accept = Some(accept);
    key
}

fn key() -> ProxyKey {
    key_of(1, "index/se/rd/serde")
}

fn at(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
}

fn document(text: &'static [u8], etag: Option<&str>) -> Cached {
    Cached {
        fetched_at: at(1_000),
        state: CachedState::Document {
            body: Bytes::from_static(text),
            etag: etag.map(|raw| ETag::new(raw.to_owned())),
            unchanged: 2,
            max_age: None,
        },
    }
}

const KEEP: Duration = Duration::from_secs(60);

/// Проверяет реализацию `ProxyCache`; `new` создаёт чистый кэш для каждого сценария.
pub async fn proxy_cache_contract<C: ProxyCache>(new: impl Fn() -> C) {
    get_of_unknown_key_is_none(&new()).await;
    put_then_get_returns_equal_entry(&new()).await;
    second_put_overwrites_first(&new()).await;
    keys_differ_by_repository_path_and_accept(&new()).await;
    missing_entry_survives_round_trip(&new()).await;
    blob_entry_survives_round_trip(&new()).await;
    document_max_age_survives_round_trip(&new()).await;
}

/// Проверяет реализацию `FetchLeases`; `new` создаёт чистый координатор для каждого сценария.
pub async fn fetch_leases_contract<L: FetchLeases>(new: impl Fn() -> L) {
    claim_of_free_key_leads(&new()).await;
    second_claim_waits_for_release_and_then_settles(&new()).await;
    claim_after_release_leads_again(&new()).await;
    release_takes_effect_without_being_polled(&new()).await;
    different_keys_are_independent(&new()).await;
    keys_differing_only_by_accept_are_independent(&new()).await;
    consecutive_leaders_get_different_tokens(&new()).await;
}

async fn get_of_unknown_key_is_none<C: ProxyCache>(cache: &C) {
    // Arrange
    let unknown = key();

    // Act
    let result = cache.get(&unknown).await;

    // Assert
    assert_eq!(
        result.expect("get"),
        None,
        "неизвестный ключ — промах, а не ошибка"
    );
}

async fn put_then_get_returns_equal_entry<C: ProxyCache>(cache: &C) {
    // Arrange
    let entry = document(b"{}", Some("\"v1\""));
    cache.put(&key(), entry.clone(), KEEP).await.expect("put");

    // Act
    let result = cache.get(&key()).await;

    // Assert
    assert_eq!(
        result.expect("get"),
        Some(entry),
        "get отдаёт ту же запись: тело, etag, счётчик и время получения"
    );
}

async fn second_put_overwrites_first<C: ProxyCache>(cache: &C) {
    // Arrange
    cache
        .put(&key(), document(b"old", None), KEEP)
        .await
        .expect("первый put");
    let newer = document(b"new", Some("\"v2\""));

    // Act
    let put = cache.put(&key(), newer.clone(), KEEP).await;

    // Assert
    put.expect("второй put");
    assert_eq!(
        cache.get(&key()).await.expect("get"),
        Some(newer),
        "повторный put заменяет запись целиком"
    );
}

async fn keys_differ_by_repository_path_and_accept<C: ProxyCache>(cache: &C) {
    // Arrange
    let base = key_of(1, "a/b");
    let other_repository = key_of(2, "a/b");
    let other_path = key_of(1, "a/c");
    let other_accept = with_accept(base.clone(), JSON);
    cache
        .put(&base, document(b"base", None), KEEP)
        .await
        .expect("put");

    // Act
    let found = (
        cache.get(&other_repository).await.expect("get репозитория"),
        cache.get(&other_path).await.expect("get пути"),
        cache.get(&other_accept).await.expect("get представления"),
    );

    // Assert
    assert_eq!(
        found,
        (None, None, None),
        "запись видна только по ключу с тем же репозиторием, путём и представлением"
    );
}

async fn document_max_age_survives_round_trip<C: ProxyCache>(cache: &C) {
    // Arrange
    let entry = Cached {
        fetched_at: at(1_000),
        state: CachedState::Document {
            body: Bytes::from_static(b"{}"),
            etag: Some(ETag::new("\"v1\"".to_owned())),
            unchanged: 2,
            max_age: Some(Duration::from_secs(600)),
        },
    };
    cache.put(&key(), entry.clone(), KEEP).await.expect("put");

    // Act
    let result = cache.get(&key()).await;

    // Assert
    assert_eq!(
        result.expect("get"),
        Some(entry),
        "объявленный upstream срок свежести переживает put/get без потерь"
    );
}

async fn missing_entry_survives_round_trip<C: ProxyCache>(cache: &C) {
    // Arrange
    let entry = Cached {
        fetched_at: at(2_000),
        state: CachedState::Missing { misses: 3 },
    };
    cache.put(&key(), entry.clone(), KEEP).await.expect("put");

    // Act
    let result = cache.get(&key()).await;

    // Assert
    assert_eq!(
        result.expect("get"),
        Some(entry),
        "отрицательная запись хранит счётчик промахов"
    );
}

async fn blob_entry_survives_round_trip<C: ProxyCache>(cache: &C) {
    // Arrange
    let entry = Cached {
        fetched_at: at(3_000),
        state: CachedState::Blob(Digests {
            sha256: Sha256::from([0x5a; 32]),
            sha512: Sha512::from([0x5b; 64]),
            sha1: Sha1::from([0x5c; 20]),
            size: 42,
        }),
    };
    cache.put(&key(), entry.clone(), KEEP).await.expect("put");

    // Act
    let result = cache.get(&key()).await;

    // Assert
    assert_eq!(
        result.expect("get"),
        Some(entry),
        "запись об артефакте хранит дайджесты"
    );
}

async fn claim_of_free_key_leads<L: FetchLeases>(leases: &L) {
    // Arrange
    let wanted = key();

    // Act
    let result = leases.claim(&wanted, LEASE, PATIENCE).await;

    // Assert
    let claim = result.expect("claim");
    assert!(
        matches!(&claim, Claim::Lead(lease) if *lease.key() == wanted),
        "свободный ключ достаётся первому претенденту вместе с lease на этот ключ: {claim:?}"
    );
}

// Второй claim «не завершается» проверяется одним опросом с no-op waker: без рантайма и sleep.
async fn second_claim_waits_for_release_and_then_settles<L: FetchLeases>(leases: &L) {
    // Arrange
    let Claim::Lead(first) = leases
        .claim(&key(), LEASE, PATIENCE)
        .await
        .expect("claim 1")
    else {
        panic!("первый claim свободного ключа обязан стать лидером");
    };
    let waiting_key = key();
    let mut second = pin!(leases.claim(&waiting_key, LEASE, PATIENCE));
    let mut cx = Context::from_waker(noop_waker_ref());
    let polled = second.as_mut().poll(&mut cx).is_pending();

    // Act
    let released = leases.release(first).await;

    // Assert
    released.expect("release");
    assert!(polled, "пока лидер держит ключ, второй claim ждёт");
    assert_eq!(
        second.await.expect("claim 2"),
        Claim::Settled,
        "после release ждущий получает Settled и перечитывает кэш"
    );
}

async fn claim_after_release_leads_again<L: FetchLeases>(leases: &L) {
    // Arrange
    let Claim::Lead(first) = leases
        .claim(&key(), LEASE, PATIENCE)
        .await
        .expect("claim 1")
    else {
        panic!("первый claim свободного ключа обязан стать лидером");
    };
    leases.release(first).await.expect("release");

    // Act
    let result = leases.claim(&key(), LEASE, PATIENCE).await;

    // Assert
    assert!(
        matches!(result.expect("claim 2"), Claim::Lead(_)),
        "после release ключ свободен: следующий претендент снова лидер"
    );
}

async fn release_takes_effect_without_being_polled<L: FetchLeases>(leases: &L) {
    // Arrange
    let Claim::Lead(first) = leases
        .claim(&key(), LEASE, PATIENCE)
        .await
        .expect("claim 1")
    else {
        panic!("первый claim свободного ключа обязан стать лидером");
    };
    drop(leases.release(first));

    // Act
    let result = leases.claim(&key(), LEASE, PATIENCE).await;

    // Assert
    assert!(
        matches!(result.expect("claim 2"), Claim::Lead(_)),
        "снятие начинается при вызове: брошенный (неопрошенный) future release ключ освобождает"
    );
}

async fn different_keys_are_independent<L: FetchLeases>(leases: &L) {
    // Arrange
    let held = key_of(1, "a/b");
    let other = key_of(1, "a/c");
    let _held = leases.claim(&held, LEASE, PATIENCE).await.expect("claim A");

    // Act
    let result = leases.claim(&other, LEASE, PATIENCE).await;

    // Assert
    assert!(
        matches!(result.expect("claim B"), Claim::Lead(_)),
        "lease на один ключ не мешает другому"
    );
}

async fn keys_differing_only_by_accept_are_independent<L: FetchLeases>(leases: &L) {
    // Arrange
    let plain = key_of(1, "simple/x/");
    let json = with_accept(plain.clone(), JSON);
    let _held = leases
        .claim(&plain, LEASE, PATIENCE)
        .await
        .expect("claim A");

    // Act
    let result = leases.claim(&json, LEASE, PATIENCE).await;

    // Assert
    assert!(
        matches!(result.expect("claim B"), Claim::Lead(_)),
        "lease на одно представление не мешает другому"
    );
}

async fn consecutive_leaders_get_different_tokens<L: FetchLeases>(leases: &L) {
    // Arrange
    let Claim::Lead(first) = leases
        .claim(&key(), LEASE, PATIENCE)
        .await
        .expect("claim 1")
    else {
        panic!("первый claim свободного ключа обязан стать лидером");
    };
    let first_token = first.token();
    leases.release(first).await.expect("release");

    // Act
    let result = leases.claim(&key(), LEASE, PATIENCE).await;

    // Assert
    let Claim::Lead(second) = result.expect("claim 2") else {
        panic!("после release ключ свободен");
    };
    assert_ne!(
        second.token(),
        first_token,
        "токены разных держателей различаются: иначе фенсинг не отличит старого лидера от нового"
    );
}
