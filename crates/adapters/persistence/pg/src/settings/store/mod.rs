//! `SettingsStore` в PostgreSQL: CAS по версии, секреты только в зашифрованном виде.

mod rows;

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crane_domain::access::Principal;
use crane_domain::settings::{Stored, StoredValue, Version};
use crane_ports::settings::{SettingsError, SettingsStore};
use crane_ports::system::Clock;
use sqlx::PgPool;

use self::rows::{Columns, backend, stored_from_row};
use super::crypto::SettingsKey;
use super::db::{driver, inconsistent, to_db_time};

const SELECT_ALL: &str =
    "SELECT key, value_text, value_sealed, key_id, version, updated_at, updated_by
     FROM setting ORDER BY key";

const INSERT: &str = "INSERT INTO setting
    (key, value_text, value_sealed, key_id, version, updated_at, updated_by)
    VALUES ($1, $2, $3, $4, $5, $6, $7)
    ON CONFLICT (key) DO NOTHING";

// Секрет и обычное значение взаимно обнуляют колонки друг друга: смена вида не оставляет
// в строке остатка прежнего значения.
const UPDATE: &str = "UPDATE setting
    SET value_text = $2, value_sealed = $3, key_id = $4, version = $5, updated_at = $6,
        updated_by = $7
    WHERE key = $1 AND version = $8";

const DELETE: &str = "DELETE FROM setting WHERE key = $1 AND version = $2";

/// Версия в `bigint`: выше `i64::MAX` не помещается, это ошибка адаптера, а не паника.
fn db_version(version: Version) -> Result<i64, SettingsError> {
    i64::try_from(version.get())
        .map_err(|_| backend(inconsistent("версия настройки не помещается в bigint")))
}

/// Настройки в PostgreSQL. Секреты шифруются [`SettingsKey`]; без ключа адаптер
/// отказывает в записи секрета и не может прочитать запечатанные строки.
/// Схему создаёт [`crate::migrate`].
pub struct PgSettings {
    pool: PgPool,
    clock: Arc<dyn Clock>,
    key: Option<SettingsKey>,
}

impl fmt::Debug for PgSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Внутри ключ шифрования, поэтому содержимое не печатается.
        f.debug_struct("PgSettings").finish_non_exhaustive()
    }
}

impl PgSettings {
    #[must_use]
    pub const fn new(pool: PgPool, clock: Arc<dyn Clock>, key: Option<SettingsKey>) -> Self {
        Self { pool, clock, key }
    }

    /// Версия, которая будет записана: первая для новой строки, следующая для обновления.
    fn written_version(expected: Option<Version>) -> Result<Version, SettingsError> {
        expected.map_or(Ok(Version::FIRST), |current| {
            current
                .next()
                .ok_or_else(|| backend(inconsistent("версия настройки переполнена")))
        })
    }
}

impl SettingsStore for PgSettings {
    async fn list(&self) -> Result<BTreeMap<String, Stored>, SettingsError> {
        let rows = sqlx::query(SELECT_ALL)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| backend(driver(error)))?;
        rows.iter()
            .map(|row| stored_from_row(row, self.key.as_ref()))
            .collect()
    }

    async fn put(
        &self,
        key: &str,
        value: StoredValue,
        expected: Option<Version>,
        by: &Principal,
    ) -> Result<Version, SettingsError> {
        let version = Self::written_version(expected)?;
        // Версия ожидаемой строки не больше записываемой, поэтому отдельной проверки не нужно.
        let written = db_version(version)?;
        let columns = Columns::encode(key, value, version, self.key.as_ref())?;
        let at = to_db_time(self.clock.now()).map_err(backend)?;
        let (sql, current) = match expected {
            None => (INSERT, None),
            Some(current) => (UPDATE, Some(db_version(current)?)),
        };
        // У INSERT и UPDATE общие $1..$7; ожидаемая версия ($8) есть только у UPDATE.
        let mut query =
            columns.bind_to(sqlx::query(sql), key.to_owned(), written, at, by.id.clone());
        if let Some(current) = current {
            query = query.bind(current);
        }
        let affected = query
            .execute(&self.pool)
            .await
            .map_err(|error| backend(driver(error)))?
            .rows_affected();
        if affected == 0 {
            return Err(SettingsError::Conflict);
        }
        Ok(version)
    }

    async fn delete(&self, key: &str, expected: Version) -> Result<(), SettingsError> {
        let affected = sqlx::query(DELETE)
            .bind(key)
            .bind(db_version(expected)?)
            .execute(&self.pool)
            .await
            .map_err(|error| backend(driver(error)))?
            .rows_affected();
        if affected == 0 {
            return Err(SettingsError::Conflict);
        }
        Ok(())
    }
}
