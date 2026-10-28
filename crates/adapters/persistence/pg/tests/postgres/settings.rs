//! Настройки: секреты лежат в БД только зашифрованными, подмена и откат строки не проходят.

use std::collections::BTreeMap;

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::settings::{Secret, Stored, StoredValue, Version};
use crane_pg::CryptoError;
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

fn secret(value: &str) -> StoredValue {
    StoredValue::Secret(Secret::new(value.to_owned()))
}

/// Причина отказа `list`, если это ошибка шифрования.
fn crypto_failure(result: Result<BTreeMap<String, Stored>, SettingsError>) -> Option<CryptoError> {
    match result {
        Err(SettingsError::Backend(error)) => error.downcast_ref::<CryptoError>().cloned(),
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
async fn execute(pool: &PgPool, sql: &'static str, bind: (&str, Vec<u8>)) {
    sqlx::query(sql)
        .bind(bind.0)
        .bind(bind.1)
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
        let row = listed.get("token").expect("строка секрета");
        assert_eq!(second.ok(), first.next());
        assert!(matches!(&row.value, StoredValue::Secret(value) if value.expose() == "new"));
    });
}

#[test]
fn list_rejects_sealed_value_copied_to_another_setting() {
    support::run(async |server| {
        // Arrange
        let (store, pool) = server.fresh_settings(Some(settings_key(1))).await;
        store
            .put("token_a", secret("first"), None, &operator())
            .await
            .expect("секрет a");
        store
            .put("token_b", secret("second"), None, &operator())
            .await
            .expect("секрет b");
        let stolen = sealed_of(&pool, "token_a").await;
        execute(
            &pool,
            "UPDATE setting SET value_sealed = $2 WHERE key = $1",
            ("token_b", stolen),
        )
        .await;

        // Act
        let result = store.list().await;

        // Assert
        assert_eq!(
            crypto_failure(result),
            Some(CryptoError::Corrupted {
                setting: "token_b".to_owned()
            })
        );
    });
}

#[test]
fn list_rejects_old_ciphertext_written_back_under_current_version() {
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
        execute(
            &pool,
            "UPDATE setting SET value_sealed = $2 WHERE key = $1",
            ("token", old),
        )
        .await;

        // Act
        let result = store.list().await;

        // Assert
        assert_eq!(
            crypto_failure(result),
            Some(CryptoError::Corrupted {
                setting: "token".to_owned()
            })
        );
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
        let result = store
            .put(
                "blobs",
                StoredValue::Plain("s3://a".to_owned()),
                None,
                &operator(),
            )
            .await;

        // Assert
        let text: Option<String> =
            sqlx::query_scalar("SELECT value_text FROM setting WHERE key = 'blobs'")
                .fetch_one(&pool)
                .await
                .expect("строка настройки");
        assert_eq!(result.ok(), Some(Version::FIRST));
        assert_eq!(text, Some("s3://a".to_owned()));
    });
}

#[test]
fn list_sealed_row_without_key_reports_missing_key_for_that_setting() {
    support::run(async |server| {
        // Arrange
        let (sealing, pool) = server.fresh_settings(Some(settings_key(1))).await;
        sealing
            .put("token", secret(PLAIN), None, &operator())
            .await
            .expect("секрет");
        let keyless = crane_pg::PgSettings::new(pool, clock(), None);

        // Act
        let result = keyless.list().await;

        // Assert
        assert_eq!(
            crypto_failure(result),
            Some(CryptoError::MissingKey {
                setting: "token".to_owned()
            })
        );
    });
}

#[test]
fn list_sealed_row_with_other_key_reports_both_fingerprints() {
    support::run(async |server| {
        // Arrange
        let (sealing, pool) = server.fresh_settings(Some(settings_key(1))).await;
        sealing
            .put("token", secret(PLAIN), None, &operator())
            .await
            .expect("секрет");
        let other = settings_key(2);
        let (stored_key_id, current_key_id) =
            (settings_key(1).id().to_owned(), other.id().to_owned());
        let reader = crane_pg::PgSettings::new(pool, clock(), Some(other));

        // Act
        let result = reader.list().await;

        // Assert
        assert_eq!(
            crypto_failure(result),
            Some(CryptoError::WrongKey {
                setting: "token".to_owned(),
                stored_key_id,
                current_key_id,
            })
        );
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
            .put(
                "blobs",
                StoredValue::Plain("x".to_owned()),
                Some(beyond),
                &operator(),
            )
            .await;

        // Assert
        assert!(matches!(result, Err(SettingsError::Backend(_))));
    });
}
