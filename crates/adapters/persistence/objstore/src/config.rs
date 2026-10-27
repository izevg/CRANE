use std::sync::Arc;

use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey};
use object_store::local::LocalFileSystem;
use object_store::prefix::PrefixStore;
use object_store::{Error, ObjectStore};
use url::Url;

use crate::ObjectBlobStore;

fn not_supported(reason: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Error {
    Error::NotSupported {
        source: reason.into(),
    }
}

impl ObjectBlobStore {
    /// Собирает хранилище по URL из `CRANE_BLOBS`.
    ///
    /// * `file:///data/blobs` — локальная ФС; каталог создаётся, если его нет. Параметры запроса не
    ///   допускаются. Запись идёт с fsync: без него после сбоя в PG осталась бы ссылка на blob,
    ///   которого нет на диске.
    /// * `s3://bucket[/prefix]?key=value&...` — S3-совместимое хранилище. Учётные данные и регион
    ///   берутся из окружения (`AWS_*`), каждая пара запроса — ключ `AmazonS3ConfigKey`. Неизвестный
    ///   ключ — ошибка, а не молчаливый пропуск. Для SeaweedFS/MinIO: `endpoint=http://host:8333`
    ///   и `aws_allow_http=true` (для `http://`).
    ///
    /// # Errors
    ///
    /// `NotSupported` для другой схемы, параметров у `file://` и URL без бакета; ошибки
    /// конфигурации и ФС — как их вернул `object_store`.
    pub fn from_url(url: &Url) -> Result<Self, Error> {
        let store = match url.scheme() {
            "file" => local(url)?,
            "s3" => s3(url)?,
            other => return Err(not_supported(format!("схема {other}:// не поддерживается"))),
        };
        Ok(Self::new(store))
    }
}

fn local(url: &Url) -> Result<Arc<dyn ObjectStore>, Error> {
    if url.query().is_some() {
        return Err(not_supported("file:// не принимает параметры запроса"));
    }
    let dir = url
        .to_file_path()
        .map_err(|()| not_supported("file:// URL должен быть абсолютным путём без хоста"))?;
    // new_with_prefix канонизирует путь и падает на отсутствующем каталоге.
    std::fs::create_dir_all(&dir).map_err(|e| Error::Generic {
        store: "LocalFileSystem",
        source: Box::new(e),
    })?;
    Ok(Arc::new(
        LocalFileSystem::new_with_prefix(dir)?.with_fsync(true),
    ))
}

fn s3(url: &Url) -> Result<Arc<dyn ObjectStore>, Error> {
    let bucket = url
        .host_str()
        .ok_or_else(|| not_supported("в s3:// URL не указан бакет"))?;
    // from_env, а не parse_url_opts: тот игнорирует учётные данные из окружения
    // и молча отбрасывает неизвестные ключи.
    let mut builder = AmazonS3Builder::from_env().with_url(format!("s3://{bucket}"));
    for (key, value) in url.query_pairs() {
        builder = builder.with_config(key.parse::<AmazonS3ConfigKey>()?, value.into_owned());
    }
    // `s3://bucket/prefix` кладёт всё под `prefix/`. Path отбрасывает пустые сегменты,
    // поэтому пустой путь или `/` даёт пустой префикс, а PrefixStore с ним ничего не меняет.
    Ok(Arc::new(PrefixStore::new(builder.build()?, url.path())))
}

#[cfg(test)]
mod tests {
    use object_store::ObjectStoreExt;
    use object_store::memory::InMemory;
    use object_store::path::Path;

    use super::*;

    #[test]
    fn s3_prefix_roots_all_cas_keys() {
        // Arrange
        let inner = Arc::new(InMemory::new());
        let store = PrefixStore::new(inner.clone(), "/team/cas/");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime");

        // Act
        let put = runtime.block_on(store.put(&Path::from("blobs/x"), "data".into()));

        // Assert
        let stored = runtime.block_on(inner.head(&Path::from("team/cas/blobs/x")));
        assert!(put.is_ok());
        assert!(stored.is_ok());
    }

    #[test]
    fn empty_or_slash_path_leaves_keys_untouched() {
        // Arrange
        let inner = Arc::new(InMemory::new());
        let store = PrefixStore::new(inner.clone(), "/");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime");

        // Act
        let put = runtime.block_on(store.put(&Path::from("blobs/x"), "data".into()));

        // Assert
        let stored = runtime.block_on(inner.head(&Path::from("blobs/x")));
        assert!(put.is_ok());
        assert!(stored.is_ok());
    }
}
