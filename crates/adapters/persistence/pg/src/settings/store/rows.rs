//! Отображение строки `setting` в доменный тип и обратно. SQL здесь нет.

use std::time::SystemTime;

use crane_domain::access::Principal;
use crane_domain::settings::{Secret, Stored, StoredValue, Version};
use crane_ports::settings::SettingsError;

use sqlx::postgres::{PgArguments, PgRow};
use sqlx::query::Query;
use sqlx::{Postgres, Row};
use time::OffsetDateTime;

use crate::db_time::{from_db_time, to_db_time};
use crate::settings::BoxError;
use crate::settings::crypto::{CryptoError, SettingsKey, open, seal};

/// Единственная воронка ошибок адаптера: драйвер, шифр или описание рассинхрона схемы.
pub(super) fn backend(error: impl Into<BoxError>) -> SettingsError {
    SettingsError::Backend(error.into())
}

/// Версия в `bigint`: выше `i64::MAX` не помещается, это ошибка адаптера, а не паника.
pub(super) fn db_version(version: Version) -> Result<i64, SettingsError> {
    i64::try_from(version.get()).map_err(|_| backend("версия настройки не помещается в bigint"))
}

/// Записываемая строка целиком в порядке `$1..$7` вставки и обновления. Значение: ровно одна из
/// `text` и `sealed` заполнена, `key_id` есть у `sealed`.
pub(super) struct Columns {
    key: String,
    text: Option<String>,
    sealed: Option<Vec<u8>>,
    key_id: Option<String>,
    version: i64,
    at: OffsetDateTime,
    by: String,
}

impl Columns {
    /// Готовит строку к записи; секрет запечатывается той же `version`, что попадёт в строку.
    pub(super) fn encode(
        setting: &str,
        value: StoredValue,
        version: Version,
        at: SystemTime,
        by: &Principal,
        cipher: Option<&SettingsKey>,
    ) -> Result<Self, SettingsError> {
        let (text, sealed, key_id) = match value {
            StoredValue::Plain(text) => (Some(text), None, None),
            StoredValue::Secret(secret) => {
                let cipher = cipher.ok_or(SettingsError::SecretsUnavailable)?;
                let sealed = seal(cipher, setting, version, &secret).map_err(backend)?;
                (None, Some(sealed), Some(cipher.id().to_owned()))
            }
        };
        Ok(Self {
            key: setting.to_owned(),
            text,
            sealed,
            key_id,
            version: db_version(version)?,
            at: to_db_time(at)
                .ok_or_else(|| backend("момент времени вне диапазона timestamptz"))?,
            by: by.id.clone(),
        })
    }

    pub(super) fn bind_to(
        self,
        query: Query<'_, Postgres, PgArguments>,
    ) -> Query<'_, Postgres, PgArguments> {
        query
            .bind(self.key)
            .bind(self.text)
            .bind(self.sealed)
            .bind(self.key_id)
            .bind(self.version)
            .bind(self.at)
            .bind(self.by)
    }
}

/// Читает колонку; ошибка драйвера не содержит значений.
fn column<'r, T>(row: &'r PgRow, name: &str) -> Result<T, SettingsError>
where
    T: sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
{
    row.try_get(name).map_err(backend)
}

/// Строка БД -> запись порта; запечатанное значение открывается ключом.
pub(super) fn stored_from_row(
    row: &PgRow,
    key: Option<&SettingsKey>,
) -> Result<(String, Stored), SettingsError> {
    let setting: String = column(row, "key")?;
    let raw_version: i64 = column(row, "version")?;
    let version = u64::try_from(raw_version)
        .ok()
        .and_then(Version::new)
        .ok_or_else(|| backend("версия настройки в БД меньше 1"))?;
    let value = match (
        column::<Option<String>>(row, "value_text")?,
        column::<Option<Vec<u8>>>(row, "value_sealed")?,
        column::<Option<String>>(row, "key_id")?,
    ) {
        (Some(text), None, None) => StoredValue::Plain(text),
        (None, Some(sealed), Some(key_id)) => {
            StoredValue::Secret(open_secret(key, &setting, version, &key_id, &sealed)?)
        }
        _ => return Err(backend("строка настройки нарушает схему")),
    };
    let stored = Stored {
        value,
        version,
        updated_at: from_db_time(column::<OffsetDateTime>(row, "updated_at")?)
            .ok_or_else(|| backend("момент времени вне диапазона SystemTime"))?,
        updated_by: column(row, "updated_by")?,
    };
    Ok((setting, stored))
}

fn open_secret(
    key: Option<&SettingsKey>,
    setting: &str,
    version: Version,
    key_id: &str,
    sealed: &[u8],
) -> Result<Secret, SettingsError> {
    let key = key.ok_or_else(|| {
        backend(CryptoError::MissingKey {
            setting: setting.to_owned(),
        })
    })?;
    open(key, setting, version, key_id, sealed).map_err(backend)
}
