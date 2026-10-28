//! Контракт `OnboardingStore`: идемпотентный `begin`, необратимый `complete`, `rotate` и `reset`.

use std::time::{Duration, SystemTime};

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::onboarding::{Onboarding, TokenHash};
use crane_domain::settings::Secret;

use crate::settings::{OnboardingError, OnboardingStore};

const NOW: SystemTime = SystemTime::UNIX_EPOCH;
const TTL: Duration = Duration::from_secs(3_600);

/// Проверяет реализацию `OnboardingStore`; `new` создаёт чистое хранилище для сценария.
pub async fn onboarding_store_contract<S: OnboardingStore>(new: impl Fn() -> S) {
    get_on_empty_store_is_none(&new()).await;
    begin_on_empty_store_returns_initial(&new()).await;
    second_begin_returns_first_state(&new()).await;
    complete_from_pending_records_who_and_when(&new()).await;
    complete_from_completed_fails_and_keeps_state(&new()).await;
    complete_without_row_is_not_started(&new()).await;
    rotate_from_pending_replaces_token(&new()).await;
    rotate_from_completed_fails_and_keeps_state(&new()).await;
    rotate_without_row_is_not_started(&new()).await;
    reset_from_completed_reopens_with_new_token(&new()).await;
    reset_without_row_is_not_started(&new()).await;
}

fn secret(raw: &str) -> Secret {
    Secret::new(raw.to_owned())
}

fn hash(raw: &str) -> TokenHash {
    TokenHash::of(&secret(raw))
}

fn pending(raw: &str) -> Onboarding {
    Onboarding::Pending {
        token: hash(raw),
        expires_at: NOW + TTL,
    }
}

fn operator(id: &str) -> Principal {
    Principal {
        id: id.to_owned(),
        kind: PrincipalKind::Human,
    }
}

/// Принимает ли хранилище токен в момент `NOW`.
async fn accepts<S: OnboardingStore>(store: &S, raw: &str) -> bool {
    store
        .get()
        .await
        .expect("get в проверке сценария")
        .is_some_and(|state| state.accepts(&secret(raw), NOW))
}

async fn get_on_empty_store_is_none<S: OnboardingStore>(store: &S) {
    // Arrange
    // Хранилище чистое: подготовки нет.

    // Act
    let state = store.get().await;

    // Assert
    assert!(state.expect("get").is_none());
}

async fn begin_on_empty_store_returns_initial<S: OnboardingStore>(store: &S) {
    // Arrange
    let initial = pending("first");

    // Act
    let result = store.begin(initial).await;

    // Assert
    let returned = result.expect("begin");
    assert!(returned.accepts(&secret("first"), NOW));
    assert!(accepts(store, "first").await);
}

async fn second_begin_returns_first_state<S: OnboardingStore>(store: &S) {
    // Arrange
    store.begin(pending("first")).await.expect("первый begin");

    // Act
    let result = store.begin(pending("second")).await;

    // Assert
    let returned = result.expect("begin");
    assert!(returned.accepts(&secret("first"), NOW));
    assert!(!returned.accepts(&secret("second"), NOW));
    assert!(accepts(store, "first").await);
}

async fn complete_from_pending_records_who_and_when<S: OnboardingStore>(store: &S) {
    // Arrange
    store.begin(pending("first")).await.expect("begin");
    let at = NOW + Duration::from_secs(5);

    // Act
    let result = store.complete(&operator("alice"), at).await;

    // Assert
    result.expect("complete");
    let state = store.get().await.expect("get").expect("строка есть");
    assert!(matches!(
        state,
        Onboarding::Completed { at: done, by } if done == at && by == "alice"
    ));
}

async fn complete_from_completed_fails_and_keeps_state<S: OnboardingStore>(store: &S) {
    // Arrange
    store.begin(pending("first")).await.expect("begin");
    let at = NOW + Duration::from_secs(5);
    store
        .complete(&operator("alice"), at)
        .await
        .expect("первый complete");

    // Act
    let result = store
        .complete(&operator("bob"), at + Duration::from_secs(1))
        .await;

    // Assert
    assert!(matches!(result, Err(OnboardingError::Completed)));
    let state = store.get().await.expect("get").expect("строка есть");
    assert!(matches!(
        state,
        Onboarding::Completed { at: done, by } if done == at && by == "alice"
    ));
}

async fn complete_without_row_is_not_started<S: OnboardingStore>(store: &S) {
    // Arrange
    // Хранилище чистое: подготовки нет.

    // Act
    let result = store.complete(&operator("alice"), NOW).await;

    // Assert
    assert!(matches!(result, Err(OnboardingError::NotStarted)));
    assert!(store.get().await.expect("get").is_none());
}

async fn rotate_from_pending_replaces_token<S: OnboardingStore>(store: &S) {
    // Arrange
    store.begin(pending("old")).await.expect("begin");

    // Act
    let result = store.rotate(hash("new"), NOW + TTL).await;

    // Assert
    result.expect("rotate");
    assert!(!accepts(store, "old").await);
    assert!(accepts(store, "new").await);
}

async fn rotate_from_completed_fails_and_keeps_state<S: OnboardingStore>(store: &S) {
    // Arrange
    store.begin(pending("old")).await.expect("begin");
    store
        .complete(&operator("alice"), NOW)
        .await
        .expect("complete");

    // Act
    let result = store.rotate(hash("new"), NOW + TTL).await;

    // Assert
    assert!(matches!(result, Err(OnboardingError::Completed)));
    assert!(
        store
            .get()
            .await
            .expect("get")
            .is_some_and(|state| state.is_completed())
    );
}

async fn rotate_without_row_is_not_started<S: OnboardingStore>(store: &S) {
    // Arrange
    // Хранилище чистое: подготовки нет.

    // Act
    let result = store.rotate(hash("new"), NOW + TTL).await;

    // Assert
    assert!(matches!(result, Err(OnboardingError::NotStarted)));
    assert!(store.get().await.expect("get").is_none());
}

async fn reset_from_completed_reopens_with_new_token<S: OnboardingStore>(store: &S) {
    // Arrange
    store.begin(pending("old")).await.expect("begin");
    store
        .complete(&operator("alice"), NOW)
        .await
        .expect("complete");

    // Act
    let result = store.reset(hash("new"), NOW + TTL).await;

    // Assert
    result.expect("reset");
    assert!(accepts(store, "new").await);
    assert!(!accepts(store, "old").await);
}

async fn reset_without_row_is_not_started<S: OnboardingStore>(store: &S) {
    // Arrange
    // Хранилище чистое: подготовки нет.

    // Act
    let result = store.reset(hash("new"), NOW + TTL).await;

    // Assert
    assert!(matches!(result, Err(OnboardingError::NotStarted)));
    assert!(store.get().await.expect("get").is_none());
}
