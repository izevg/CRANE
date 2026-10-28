//! Контракт `SettingsStore`: CAS по версии, удаление и секреты.

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::settings::{NewValue, Secret, Stored, StoredValue, Version};

use crate::settings::{SettingsError, SettingsStore};

/// Проверяет реализацию `SettingsStore`; `new` создаёт чистое хранилище для сценария.
pub async fn settings_store_contract<S: SettingsStore>(new: impl Fn() -> S) {
    put_without_expected_on_empty_store_creates_first_version(&new()).await;
    put_without_expected_on_existing_key_conflicts(&new()).await;
    put_with_stale_version_conflicts(&new()).await;
    put_with_current_version_bumps_version_and_value(&new()).await;
    put_with_version_of_missing_key_conflicts(&new()).await;
    delete_with_current_version_removes_row(&new()).await;
    delete_with_stale_version_conflicts_and_keeps_row(&new()).await;
    put_secret_is_listed_as_secret_with_same_value(&new()).await;
}

fn operator() -> Principal {
    Principal {
        id: "alice".to_owned(),
        kind: PrincipalKind::Human,
    }
}

fn plain(value: &str) -> NewValue {
    NewValue::Plain(value.to_owned())
}

async fn row<S: SettingsStore>(store: &S, key: &str) -> Option<Stored> {
    store
        .list()
        .await
        .expect("list в проверке сценария")
        .remove(key)
}

fn is_plain(stored: &Stored, expected: &str) -> bool {
    matches!(&stored.value, StoredValue::Plain(value) if value == expected)
}

/// Кладёт строку и возвращает её версию.
async fn seed<S: SettingsStore>(store: &S, key: &str, value: &str) -> Version {
    store
        .put(key, plain(value), None, &operator())
        .await
        .expect("put в подготовке сценария")
}

async fn put_without_expected_on_empty_store_creates_first_version<S: SettingsStore>(store: &S) {
    // Arrange
    let by = operator();

    // Act
    let result = store.put("blobs", plain("s3://a"), None, &by).await;

    // Assert
    assert_eq!(result.expect("put"), Version::FIRST);
    let stored = row(store, "blobs").await.expect("строка есть в list");
    assert!(is_plain(&stored, "s3://a"));
    assert_eq!(stored.version, Version::FIRST);
    assert_eq!(stored.updated_by, by.id);
}

async fn put_without_expected_on_existing_key_conflicts<S: SettingsStore>(store: &S) {
    // Arrange
    seed(store, "blobs", "old").await;

    // Act
    let result = store.put("blobs", plain("new"), None, &operator()).await;

    // Assert
    assert!(matches!(result, Err(SettingsError::Conflict)));
    let stored = row(store, "blobs").await.expect("строка на месте");
    assert!(is_plain(&stored, "old"));
    assert_eq!(stored.version, Version::FIRST);
}

async fn put_with_stale_version_conflicts<S: SettingsStore>(store: &S) {
    // Arrange
    let first = seed(store, "blobs", "v1").await;
    store
        .put("blobs", plain("v2"), Some(first), &operator())
        .await
        .expect("обновление в подготовке сценария");

    // Act
    let result = store
        .put("blobs", plain("stale"), Some(first), &operator())
        .await;

    // Assert
    assert!(matches!(result, Err(SettingsError::Conflict)));
    let stored = row(store, "blobs").await.expect("строка на месте");
    assert!(is_plain(&stored, "v2"));
}

async fn put_with_current_version_bumps_version_and_value<S: SettingsStore>(store: &S) {
    // Arrange
    let first = seed(store, "blobs", "v1").await;
    let by = Principal {
        id: "bob".to_owned(),
        kind: PrincipalKind::Human,
    };

    // Act
    let result = store.put("blobs", plain("v2"), Some(first), &by).await;

    // Assert
    let next = result.expect("put");
    assert_eq!(Some(next), first.next());
    let stored = row(store, "blobs").await.expect("строка на месте");
    assert!(is_plain(&stored, "v2"));
    assert_eq!(stored.version, next);
    assert_eq!(stored.updated_by, "bob");
}

async fn put_with_version_of_missing_key_conflicts<S: SettingsStore>(store: &S) {
    // Arrange
    let version = Version::FIRST;

    // Act
    let result = store
        .put("blobs", plain("x"), Some(version), &operator())
        .await;

    // Assert
    assert!(matches!(result, Err(SettingsError::Conflict)));
    assert!(row(store, "blobs").await.is_none());
}

async fn delete_with_current_version_removes_row<S: SettingsStore>(store: &S) {
    // Arrange
    let version = seed(store, "blobs", "v1").await;

    // Act
    let result = store.delete("blobs", version).await;

    // Assert
    result.expect("delete");
    assert!(row(store, "blobs").await.is_none());
}

async fn delete_with_stale_version_conflicts_and_keeps_row<S: SettingsStore>(store: &S) {
    // Arrange
    let first = seed(store, "blobs", "v1").await;
    store
        .put("blobs", plain("v2"), Some(first), &operator())
        .await
        .expect("обновление в подготовке сценария");

    // Act
    let result = store.delete("blobs", first).await;

    // Assert
    assert!(matches!(result, Err(SettingsError::Conflict)));
    assert!(row(store, "blobs").await.is_some());
}

async fn put_secret_is_listed_as_secret_with_same_value<S: SettingsStore>(store: &S) {
    // Arrange
    let value = NewValue::Secret(Secret::new("crs_secret".to_owned()));

    // Act
    let result = store.put("token", value, None, &operator()).await;

    // Assert
    result.expect("put");
    let stored = row(store, "token").await.expect("строка есть");
    assert!(
        matches!(&stored.value, StoredValue::Secret(secret) if secret.expose() == "crs_secret")
    );
}
