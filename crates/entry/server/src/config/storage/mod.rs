//! Хранилища: каталог в PostgreSQL и blob (память, `file://`, `s3://`).

mod blobs;
mod database;

use std::io;
use std::path::Path;

use crane_objstore::S3Key;
use url::Url;

use blobs::blobs_url;
pub(crate) use database::Database;

use super::source::{required, secret};
use super::{ConfigError, MEMORY};

pub(super) const DATABASE_URL: &str = "CRANE_DATABASE_URL";
const DATABASE_URL_FILE: &str = "CRANE_DATABASE_URL_FILE";
pub(super) const BLOBS: &str = "CRANE_BLOBS";
// Ключи S3 читает object_store из окружения сам; здесь только пара `*_FILE`, которой он не знает.
// В VARS не входят: AWS_* не ключи TOML.
const S3_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
pub(super) const S3_KEY_ID_FILE: &str = "AWS_ACCESS_KEY_ID_FILE";
const S3_SECRET: &str = "AWS_SECRET_ACCESS_KEY";
pub(super) const S3_SECRET_FILE: &str = "AWS_SECRET_ACCESS_KEY_FILE";

pub(super) const VARS: [&str; 3] = [DATABASE_URL, DATABASE_URL_FILE, BLOBS];

/// Выбор хранилищ. Один режим на оба, а не два независимых выбора: смешанная пара
/// непредставима. Новый бэкенд blob (GCS, Azure) этот тип не меняет: схему разбирает `from_url`.
pub(crate) enum Storage {
    /// Данные теряются при рестарте; только оба хранилища вместе.
    Memory,
    /// Каталог и CAS переживают рестарт одинаково, иначе каталог ссылался бы на потерянные blob.
    Durable {
        database: Database,
        blobs: Url,
        /// Ключ из `AWS_*_FILE`; без файлов `None`, учётные данные берёт сам `object_store`.
        s3_key: Option<S3Key>,
    },
}

impl Storage {
    /// Оба значения обязательны, а `memory` пишется явно: молчаливое хранилище в памяти
    /// в продакшене потеряло бы данные при рестарте.
    pub(crate) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        let database = secret(var, read_file, DATABASE_URL, DATABASE_URL_FILE)?;
        let blobs = required(var, BLOBS)?;
        let s3_key = s3_key(var, read_file)?;
        match (database.expose() == MEMORY, blobs == MEMORY) {
            (true, true) if s3_key.is_some() => Err(ConfigError::S3KeyWithMemory),
            (true, true) => Ok(Self::Memory),
            (false, false) => Ok(Self::Durable {
                database: Database::parse(&database)?,
                blobs: blobs_url(&blobs)?,
                s3_key,
            }),
            _ => Err(ConfigError::MixedStorage),
        }
    }
}

/// Статический ключ S3 из файлов. Без `*_FILE` это `None`: плоские `AWS_*` и роли читает сам
/// `object_store`. Если задан хоть один файл, нужна вся пара; полноту и конфликт «переменная +
/// файл» проверяет `secret`.
fn s3_key(
    var: &impl Fn(&str) -> Option<String>,
    read_file: &impl Fn(&Path) -> io::Result<String>,
) -> Result<Option<S3Key>, ConfigError> {
    if var(S3_KEY_ID_FILE).is_none() && var(S3_SECRET_FILE).is_none() {
        return Ok(None);
    }
    let id = secret(var, read_file, S3_KEY_ID, S3_KEY_ID_FILE)?;
    let secret = secret(var, read_file, S3_SECRET, S3_SECRET_FILE)?;
    Ok(Some(S3Key::new(
        id.expose().to_owned(),
        secret.expose().to_owned(),
    )))
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
