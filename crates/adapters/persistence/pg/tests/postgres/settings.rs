//! Настройки: секреты лежат в БД только зашифрованными, подмена и откат строки не проходят,
//! нечитаемый секрет остаётся значением строки.

use std::collections::BTreeMap;

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::settings::{NewValue, Secret, Stored, StoredValue, Unreadable, Version};
use crane_pg::PgSettings;
use crane_ports::settings::{SettingsError, SettingsStore};
use sqlx::PgPool;

use crate::support;
use crate::support::settings::{clock, settings_key};

const PLAIN: &str = "crs_super-secret-value";

fn operator() -> Principal {
    Principal {
        id: "alice".to_owned(),
        kind: PrincipalKind::Human,
    }
}

fn secret(value: &str) -> NewValue {
    NewValue::Secret(Secret::new(value.to_owned()))
}

fn text(value: &str) -> NewValue {
    NewValue::Plain(value.to_owned())
}

/// Причина, по которой строка `key` не читается; `None`, если строки нет или она читается.
fn unreadable_of(listed: &BTreeMap<String, Stored>, key: &str) -> Option<Unreadable> {
    match &listed.get(key)?.value {
        StoredValue::Unreadable(reason) => Some(reason.clone()),
        _ => None,
    }
}

fn secret_text(listed: &BTreeMap<String, Stored>, key: &str) -> Option<String> {
    match &listed.get(key)?.value {
        StoredValue::Secret(value) => Some(value.expose().to_owned()),
        _ => None,
    }
}

#[allow(
    clippy::expect_used,
    reason = "помощник подготовки сценария: сбой фикстуры должен ронять тест"
)]
async fn sealed_of(pool: &PgPool, key: &str) -> Vec<u8> {
    sqlx::query_scalar("SELECT value_sealed FROM setting WHERE key = $1")
        .bind(key)
        .fetch_one(pool)
        .await
        .expect("запечатанное значение строки")
}

#[allow(
    clippy::expect_used,
    reason = "помощник подготовки сценария: сбой фикстуры должен ронять тест"
)]
async fn replace_sealed(pool: &PgPool, key: &str, sealed: Vec<u8>) {
    sqlx::query("UPDATE setting SET value_sealed = $2 WHERE key = $1")
        .bind(key)
        .bind(sealed)
        .execute(pool)
        .await
        .expect("правка строки в подготовке сценария");
}

#[test]
fn secret_is_stored_only_sealed_under_key_fingerprint() {
    support::run(async |server| {
        // Arrange
        let key = settings_key(1);
        let key_id = key.id().to_owned();
        let (store, pool) = server.fresh_settings(Some(key)).await;

        // Act
        let put = store.put("token", secret(PLAIN), None, &operator()).await;

        // Assert
        let row: (Option<String>, Option<Vec<u8>>, Option<String>) = sqlx::query_as(
            "SELECT value_text, value_sealed, key_id FROM setting WHERE key = 'token'",
        )
        .fetch_one(&pool)
        .await
        .expect("строка секрета");
        assert!(put.is_ok());
        assert_eq!(row.0, None);
        assert_eq!(row.2, Some(key_id));
        let sealed = row.1.expect("значение запечатано");
        assert!(!sealed.windows(PLAIN.len()).any(|w| w == PLAIN.as_bytes()));
    });
}

#[test]
fn secret_rewritten_with_expected_version_is_readable_under_new_version() {
    support::run(async |server| {
        // Arrange
        let (store, _pool) = server.fresh_settings(Some(settings_key(1))).await;
        let first = store
            .put("token", secret("old"), None, &operator())
            .await
            .expect("v1");

        // Act
        let second = store
            .put("token", secret("new"), Some(first), &operator())
            .await;

        // Assert
        let listed = store.list().await.expect("list");
        assert_eq!(second.ok(), first.next());
        assert_eq!(secret_text(&listed, "token"), Some("new".to_owned()));
    });
}

#[test]
fn list_marks_sealed_value_copied_to_another_setting_as_corrupted() {
    support::run(async |server| {
        // Arrange
        let (store, pool) = server.fresh_settings(Some(settings_key(1))).await;
        for (name, value) in [("token_a", "first"), ("token_b", "second")] {
            store
                .put(name, secret(value), None, &operator())
                .await
                .expect("секрет в подготовке");
        }
        replace_sealed(&pool, "token_b", sealed_of(&pool, "token_a").await).await;

        // Act
        let listed = store.list().await.expect("list не падает из-за шифра");

        // Assert
        assert_eq!(
            unreadable_of(&listed, "token_b"),
            Some(Unreadable::Corrupted)
        );
        assert_eq!(secret_text(&listed, "token_a"), Some("first".to_owned()));
    });
}

#[test]
fn list_marks_old_ciphertext_written_back_under_current_version_as_corrupted() {
    support::run(async |server| {
        // Arrange
        let (store, pool) = server.fresh_settings(Some(settings_key(1))).await;
        let first = store
            .put("token", secret("old"), None, &operator())
            .await
            .expect("v1");
        let old = sealed_of(&pool, "token").await;
        store
            .put("token", secret("new"), Some(first), &operator())
            .await
            .expect("v2");
        replace_sealed(&pool, "token", old).await;

        // Act
        let listed = store.list().await.expect("list не падает из-за шифра");

        // Assert
        assert_eq!(unreadable_of(&listed, "token"), Some(Unreadable::Corrupted));
    });
}

#[test]
fn list_marks_flipped_ciphertext_byte_as_corrupted() {
    support::run(async |server| {
        // Arrange
        let (store, pool) = server.fresh_settings(Some(settings_key(1))).await;
        store
            .put("token", secret(PLAIN), None, &operator())
            .await
            .expect("секрет");
        let mut sealed = sealed_of(&pool, "token").await;
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        replace_sealed(&pool, "token", sealed).await;

        // Act
        let listed = store.list().await.expect("list не падает из-за шифра");

        // Assert
        assert_eq!(unreadable_of(&listed, "token"), Some(Unreadable::Corrupted));
    });
}

#[test]
fn put_secret_without_key_is_unavailable_and_writes_nothing() {
    support::run(async |server| {
        // Arrange
        let (store, pool) = server.fresh_settings(None).await;

        // Act
        let result = store.put("token", secret(PLAIN), None, &operator()).await;

        // Assert
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM setting")
            .fetch_one(&pool)
            .await
            .expect("число строк");
        assert!(matches!(result, Err(SettingsError::SecretsUnavailable)));
        assert_eq!(rows, 0);
    });
}

#[test]
fn put_plain_without_key_is_stored_as_text() {
    support::run(async |server| {
        // Arrange
        let (store, pool) = server.fresh_settings(None).await;

        // Act
        let result = store.put("blobs", text("s3://a"), None, &operator()).await;

        // Assert
        let stored: Option<String> =
            sqlx::query_scalar("SELECT value_text FROM setting WHERE key = 'blobs'")
                .fetch_one(&pool)
                .await
                .expect("строка настройки");
        assert_eq!(result.ok(), Some(Version::FIRST));
        assert_eq!(stored, Some("s3://a".to_owned()));
    });
}

#[test]
fn list_without_key_reports_sealed_row_as_missing_key_and_keeps_neighbours_readable() {
    support::run(async |server| {
        // Arrange
        let (sealing, pool) = server.fresh_settings(Some(settings_key(1))).await;
        sealing
            .put("token", secret(PLAIN), None, &operator())
            .await
            .expect("секрет");
        sealing
            .put("blobs", text("s3://a"), None, &operator())
            .await
            .expect("обычное значение");
        let keyless = PgSettings::new(pool, clock(), None);

        // Act
        let listed = keyless.list().await.expect("list без ключа не падает");

        // Assert
        let token = listed.get("token").expect("строка секрета видна");
        let blobs = listed.get("blobs").expect("соседняя строка видна");
        assert_eq!(
            unreadable_of(&listed, "token"),
            Some(Unreadable::MissingKey)
        );
        assert_eq!(
            (token.version, token.updated_by.as_str()),
            (Version::FIRST, "alice")
        );
        assert!(matches!(&blobs.value, StoredValue::Plain(value) if value == "s3://a"));
    });
}

#[test]
fn list_with_other_key_reports_both_fingerprints() {
    support::run(async |server| {
        // Arrange
        let (sealing, pool) = server.fresh_settings(Some(settings_key(1))).await;
        sealing
            .put("token", secret(PLAIN), None, &operator())
            .await
            .expect("секрет");
        let other = settings_key(2);
        let expected = Unreadable::WrongKey {
            stored_key_id: settings_key(1).id().to_owned(),
            current_key_id: other.id().to_owned(),
        };
        let reader = PgSettings::new(pool, clock(), Some(other));

        // Act
        let listed = reader.list().await.expect("list с другим ключом не падает");

        // Assert
        assert_eq!(unreadable_of(&listed, "token"), Some(expected));
    });
}

#[test]
fn unreadable_secret_is_overwritten_by_cas_with_the_new_key() {
    support::run(async |server| {
        // Arrange
        let (sealing, pool) = server.fresh_settings(Some(settings_key(1))).await;
        let first = sealing
            .put("token", secret("lost"), None, &operator())
            .await
            .expect("секрет под старым ключом");
        let reader = PgSettings::new(pool, clock(), Some(settings_key(2)));

        // Act
        let result = reader
            .put("token", secret("fresh"), Some(first), &operator())
            .await;

        // Assert
        let listed = reader.list().await.expect("list");
        assert_eq!(result.ok(), first.next());
        assert_eq!(secret_text(&listed, "token"), Some("fresh".to_owned()));
    });
}

#[test]
fn plain_value_overwrites_missing_key_secret_without_a_key() {
    support::run(async |server| {
        // Arrange
        let (sealing, pool) = server.fresh_settings(Some(settings_key(1))).await;
        let first = sealing
            .put("token", secret("lost"), None, &operator())
            .await
            .expect("секрет");
        let keyless = PgSettings::new(pool.clone(), clock(), None);

        // Act
        let result = keyless
            .put("token", text("plain"), Some(first), &operator())
            .await;

        // Assert
        let row: (Option<String>, Option<Vec<u8>>, Option<String>) = sqlx::query_as(
            "SELECT value_text, value_sealed, key_id FROM setting WHERE key = 'token'",
        )
        .fetch_one(&pool)
        .await
        .expect("строка");
        assert!(result.is_ok());
        assert_eq!(row, (Some("plain".to_owned()), None, None));
    });
}

#[test]
fn put_with_expected_version_beyond_bigint_is_backend_error() {
    support::run(async |server| {
        // Arrange
        let (store, _pool) = server.fresh_settings(None).await;
        let beyond = Version::new(u64::from(u32::MAX) << 32).expect("версия не ноль");

        // Act
        let result = store
            .put("blobs", text("x"), Some(beyond), &operator())
            .await;

        // Assert
        assert!(matches!(result, Err(SettingsError::Backend(_))));
    });
}

#[test]
fn closed_pool_makes_settings_store_unreachable() {
    support::run(async |server| {
        // Arrange
        let (store, pool) = server.fresh_settings(None).await;
        pool.close().await;

        // Act
        let result = store.list().await;

        // Assert
        assert!(matches!(result, Err(SettingsError::Unreachable(_))));
    });
}
