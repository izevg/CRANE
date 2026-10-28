//! Отображение строки `setting` в доменный тип и обратно. SQL здесь нет.

use crane_domain::settings::{Secret, Stored, StoredValue, Version};
use crane_ports::settings::SettingsError;
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::query::Query;
use sqlx::{Postgres, Row};
use time::OffsetDateTime;

use crate::settings::crypto::{CryptoError, SettingsKey, open, seal};
use crate::settings::db::{BoxError, driver, from_db_time, inconsistent};

pub(super) fn backend(error: BoxError) -> SettingsError {
    SettingsError::Backend(error)
}

/// Три колонки значения: ровно одна из `text` и `sealed` заполнена, `key_id` есть у `sealed`.
pub(super) struct Columns {
    text: Option<String>,
    sealed: Option<Vec<u8>>,
    key_id: Option<String>,
}

impl Columns {
    /// Готовит колонки к записи; секрет запечатывается версией, которая будет записана.
    pub(super) fn encode(
        setting: &str,
        value: StoredValue,
        version: Version,
        key: Option<&SettingsKey>,
    ) -> Result<Self, SettingsError> {
        match value {
            StoredValue::Plain(text) => Ok(Self {
                text: Some(text),
                sealed: None,
                key_id: None,
            }),
            StoredValue::Secret(secret) => {
                let key = key.ok_or(SettingsError::SecretsUnavailable)?;
                let sealed = seal(key, setting, version, &secret)
                    .map_err(|error| backend(Box::new(error)))?;
                Ok(Self {
                    text: None,
                    sealed: Some(sealed),
                    key_id: Some(key.id().to_owned()),
                })
            }
        }
    }

    /// Привязывает $1..$7: ключ, три колонки значения, версия, время, автор.
    pub(super) fn bind_to(
        self,
        query: Query<'_, Postgres, PgArguments>,
        setting: String,
        version: i64,
        at: OffsetDateTime,
        by: String,
    ) -> Query<'_, Postgres, PgArguments> {
        query
            .bind(setting)
            .bind(self.text)
            .bind(self.sealed)
            .bind(self.key_id)
            .bind(version)
            .bind(at)
            .bind(by)
    }
}

/// Читает колонку; ошибка драйвера не содержит значений.
fn column<'r, T>(row: &'r PgRow, name: &str) -> Result<T, SettingsError>
where
    T: sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
{
    row.try_get(name).map_err(|error| backend(driver(error)))
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
        .ok_or_else(|| backend(inconsistent("версия настройки в БД меньше 1")))?;
    let value = match (
        column::<Option<String>>(row, "value_text")?,
        column::<Option<Vec<u8>>>(row, "value_sealed")?,
        column::<Option<String>>(row, "key_id")?,
    ) {
        (Some(text), None, None) => StoredValue::Plain(text),
        (None, Some(sealed), Some(key_id)) => {
            StoredValue::Secret(open_secret(key, &setting, version, &key_id, &sealed)?)
        }
        _ => return Err(backend(inconsistent("строка настройки нарушает схему"))),
    };
    let stored = Stored {
        value,
        version,
        updated_at: from_db_time(column::<OffsetDateTime>(row, "updated_at")?).map_err(backend)?,
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
        backend(Box::new(CryptoError::MissingKey {
            setting: setting.to_owned(),
        }))
    })?;
    open(key, setting, version, key_id, sealed).map_err(|error| backend(Box::new(error)))
}
