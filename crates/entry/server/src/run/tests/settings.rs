//! Старт сервера над слоем настроек в БД. Каждый тест берёт свежую БД: строки `setting`
//! общей базы мешали бы параллельным тестам (`CRANE_TEST_DATABASE_URL`, см. `stack`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap - это падение теста"
)]

use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::settings::{NewValue, Secret, Version};
use crane_pg::{PgSettings, SettingsKey};
use crane_ports::settings::{SettingsSchema, SettingsStore};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};
use tokio::runtime::Runtime;
use url::Url;
use uuid::Uuid;

use super::stack::{TOKEN, database_of, database_url, lookup, runtime, token_lookup};
use super::{load, migrate};
use crate::system::SystemClock;

/// Одна из двух разных ключей шифрования (base64 от 32 байт).
pub(super) const KEY_A: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
pub(super) const KEY_B: &str = "Hh8dHBsaGRgXFhUUExIREA8ODQwLCgkIBwYFBAMCAQA=";

pub(super) fn key_id(encoded: &str) -> String {
    SettingsKey::from_base64(&Secret::new(encoded.to_owned()))
        .unwrap()
        .id()
        .to_owned()
}

/// Свежая БД на запуск: строки `setting` не должны мешать параллельным тестам.
pub(super) struct FreshDb {
    pub(super) runtime: Runtime,
    admin: PgPool,
    name: String,
    pub(super) url: String,
}

impl FreshDb {
    pub(super) fn create(base_url: &str) -> Self {
        let runtime = runtime();
        let name = format!("crane_settings_{}", Uuid::new_v4().simple());
        let admin = runtime.block_on(async {
            let options = PgConnectOptions::from_str(base_url).unwrap();
            PgPoolOptions::new().connect_with(options).await.unwrap()
        });
        runtime
            .block_on(sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}"))).execute(&admin))
            .unwrap();
        let mut target = Url::parse(base_url).unwrap();
        target.set_path(&format!("/{name}"));
        let url = target.to_string();
        let db = Self {
            runtime,
            admin,
            name,
            url,
        };
        migrate(database_of(&lookup(&db.url, Path::new("/"), &[]))).unwrap();
        db
    }

    pub(super) fn store(&self, key: Option<&str>) -> PgSettings {
        let options = PgConnectOptions::from_str(&self.url).unwrap();
        let pool = self
            .runtime
            .block_on(PgPoolOptions::new().connect_with(options))
            .unwrap();
        let key =
            key.map(|encoded| SettingsKey::from_base64(&Secret::new(encoded.to_owned())).unwrap());
        PgSettings::new(pool, Arc::new(SystemClock), key)
    }

    pub(super) fn put(&self, store: &PgSettings, key: &str, value: NewValue) {
        let by = Principal {
            id: "admin".to_owned(),
            kind: PrincipalKind::Robot,
        };
        self.runtime
            .block_on(store.put(key, value, None, &by))
            .unwrap();
    }

    pub(super) fn drop_database(self) {
        // Если тест упал по panic, БД остаётся: чистка по префиксу убила бы БД параллельных процессов.
        self.runtime.block_on(async {
            sqlx::query(AssertSqlSafe(format!(
                "DROP DATABASE IF EXISTS {} WITH (FORCE)",
                self.name
            )))
            .execute(&self.admin)
            .await
            .unwrap();
        });
    }
}

#[test]
fn stored_value_acts_when_the_environment_is_silent_and_its_version_is_snapshotted() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    db.put(
        &db.store(None),
        "token_principal",
        NewValue::Plain("stored-principal".to_owned()),
    );
    // Снимок сырой: скрытые и неизвестные строки отсеивает app, а не entry.
    db.put(
        &db.store(None),
        "http_port",
        NewValue::Plain("1".to_owned()),
    );
    db.put(
        &db.store(None),
        "no_such_key",
        NewValue::Plain("1".to_owned()),
    );
    let var = token_lookup(&db.url, dir.path());

    // Act
    let loaded = db.runtime.block_on(load(var));

    // Assert
    let loaded = loaded.ok().unwrap();
    let observed = (
        loaded.config.principal.id.clone(),
        loaded.settings.started.clone(),
        loaded.settings.persistent(),
    );
    db.drop_database();
    assert_eq!(
        observed,
        (
            "stored-principal".to_owned(),
            BTreeMap::from([
                ("http_port".to_owned(), Version::FIRST),
                ("no_such_key".to_owned(), Version::FIRST),
                ("token_principal".to_owned(), Version::FIRST),
            ]),
            true
        )
    );
}

#[test]
fn environment_overrides_the_stored_value() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    db.put(
        &db.store(None),
        "token_principal",
        NewValue::Plain("stored-principal".to_owned()),
    );
    let var = lookup(
        &db.url,
        dir.path(),
        &[
            ("CRANE_TOKEN", TOKEN),
            ("CRANE_TOKEN_PRINCIPAL", "env-principal"),
        ],
    );

    // Act
    let loaded = db.runtime.block_on(load(var));

    // Assert
    let principal = loaded.ok().unwrap().config.principal.id;
    db.drop_database();
    assert_eq!(principal, "env-principal");
}

#[test]
fn start_fails_on_secret_sealed_with_another_key_naming_the_key_and_both_fingerprints() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    db.put(
        &db.store(Some(KEY_A)),
        "token",
        NewValue::Secret(Secret::new("stored-token".to_owned())),
    );
    let var = lookup(&db.url, dir.path(), &[("CRANE_SETTINGS_KEY", KEY_B)]);

    // Act
    let result = db.runtime.block_on(load(var));

    // Assert
    let message = result.err().unwrap().to_string();
    db.drop_database();
    assert!(message.contains("`token`"), "{message}");
    assert!(message.contains(&key_id(KEY_A)), "{message}");
    assert!(message.contains(&key_id(KEY_B)), "{message}");
}

#[test]
fn unreadable_secret_shadowed_by_the_environment_does_not_stop_the_start() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    db.put(
        &db.store(Some(KEY_A)),
        "token",
        NewValue::Secret(Secret::new("stored-token".to_owned())),
    );
    let var = lookup(
        &db.url,
        dir.path(),
        &[("CRANE_SETTINGS_KEY", KEY_B), ("CRANE_TOKEN", TOKEN)],
    );

    // Act
    let result = db.runtime.block_on(load(var));

    // Assert
    let started = result.is_ok();
    db.drop_database();
    assert!(started);
}

#[test]
fn memory_start_has_empty_settings_layer_and_exposes_the_schema() {
    // Arrange
    let runtime = runtime();
    let dir = tempfile::tempdir().unwrap();
    let boot = lookup(
        "memory",
        dir.path(),
        &[("CRANE_BLOBS", "memory"), ("CRANE_TOKEN", TOKEN)],
    );

    // Act
    let loaded = runtime.block_on(load(boot));

    // Assert
    let wiring = loaded.ok().unwrap().settings;
    assert_eq!(
        (
            wiring.persistent(),
            wiring.started.len(),
            wiring.schema.inherited("token").map(|i| i.var)
        ),
        (false, 0, Some("CRANE_TOKEN".to_owned()))
    );
}
