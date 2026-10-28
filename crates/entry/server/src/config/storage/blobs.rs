//! Blob-хранилище: разбор `CRANE_BLOBS` и ключа S3 из файлов.

use std::io;
use std::path::Path;

use crane_objstore::S3Key;
use url::Url;

use super::{BLOBS, S3_KEY_ID, S3_KEY_ID_FILE, S3_SECRET, S3_SECRET_FILE};
use crate::config::source::{required, secret};
use crate::config::{ConfigError, MEMORY};

/// Выбор blob-хранилища. Пара с каталогом проверяется здесь, а не двумя независимыми выборами:
/// смешанная пара непредставима. Новый бэкенд (GCS, Azure) тип не меняет: схему разбирает `from_url`.
pub(crate) enum Blobs {
    /// Данные теряются при рестарте; только вместе с каталогом в памяти.
    Memory,
    /// Каталог и CAS переживают рестарт одинаково, иначе каталог ссылался бы на потерянные blob.
    Durable {
        url: Url,
        /// Ключ из `AWS_*_FILE`; без файлов `None`, учётные данные берёт сам `object_store`.
        s3_key: Option<S3Key>,
    },
}

impl Blobs {
    /// Значение обязательно, а `memory` пишется явно. `persistent` — режим каталога (`Database`).
    pub(crate) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
        persistent: bool,
    ) -> Result<Self, ConfigError> {
        let blobs = required(var, BLOBS)?;
        let s3_key = s3_key(var, read_file)?;
        match (persistent, blobs == MEMORY) {
            (false, true) if s3_key.is_some() => Err(ConfigError::S3KeyWithMemory),
            (false, true) => Ok(Self::Memory),
            (true, false) => Ok(Self::Durable {
                url: blobs_url(&blobs)?,
                s3_key,
            }),
            _ => Err(ConfigError::MixedStorage),
        }
    }
}

/// Статический ключ S3 из файлов. Без `*_FILE` это `None`: плоские `AWS_*` и роли читает сам
/// `object_store`. Если задан хоть один файл, нужна вся пара; полноту и конфликт «переменная +
/// файл» проверяет `secret`. Замыкание `var` отвечает на любое имя, а не только на `vars()`.
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

pub(super) fn blobs_url(value: &str) -> Result<Url, ConfigError> {
    // Ошибка разбора и сам URL не выводятся: в запросе `s3://` бывают ключи доступа.
    let url = Url::parse(value).map_err(|_| ConfigError::Invalid {
        var: BLOBS,
        reason: "ожидалось memory, file:///абсолютный/путь или s3://бакет",
    })?;
    match url.scheme() {
        "file" | "s3" => {}
        scheme => {
            return Err(ConfigError::Unsupported {
                var: BLOBS,
                scheme: scheme.to_owned(),
            });
        }
    }
    // Ключи проверяет адаптер теми же правилами, что разбирает: в URL остаются только
    // несекретные параметры, поэтому ни URL, ни текст ошибки не процитируют секрет.
    match url
        .query_pairs()
        .find(|(key, value)| !crane_objstore::is_public_s3_param(key, value))
    {
        Some((key, _)) => Err(ConfigError::BlobsKey {
            key: key.into_owned(),
        }),
        None => Ok(url),
    }
}
