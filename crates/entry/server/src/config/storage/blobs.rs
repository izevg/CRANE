//! Разбор `CRANE_BLOBS` в URL blob-хранилища.

use url::Url;

use super::BLOBS;
use crate::config::ConfigError;

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
