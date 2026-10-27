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

/// Основные имена несекретных ключей `s3://` (как их отдаёт `AmazonS3ConfigKey::as_ref`).
/// Белый список: новый секретный ключ библиотеки (как `aws_sse_customer_key_base64`) не
/// проскочит в URL, а значит, и в журнал. Учётные данные, их поставщики (роль, токены,
/// metadata) и `proxy_url` (в нём бывает пароль) задаются только переменными AWS_*.
const PUBLIC_S3_KEYS: [&str; 28] = [
    "aws_region",
    "aws_default_region",
    "aws_bucket",
    "aws_endpoint",
    "aws_endpoint_url_s3",
    "aws_virtual_hosted_style_request",
    "aws_s3_express",
    "aws_unsigned_payload",
    "aws_checksum_algorithm",
    "aws_skip_signature",
    "aws_copy_if_not_exists",
    "aws_conditional_put",
    "aws_disable_tagging",
    "aws_disable_bulk_delete",
    "aws_request_payer",
    "aws_server_side_encryption",
    "aws_sse_kms_key_id",
    "aws_sse_bucket_key_enabled",
    "allow_http",
    "allow_invalid_certificates",
    "disable_system_certificates",
    "connect_timeout",
    "timeout",
    "read_timeout",
    "pool_idle_timeout",
    "pool_max_idle_per_host",
    "http1_only",
    "user_agent",
];

/// Параметр запроса `s3://` можно держать в URL: ключ разобран самой библиотекой (её
/// синонимы и регистр), он несекретный, а адрес endpoint не несёт логина и пароля.
#[must_use]
pub fn is_public_s3_param(key: &str, value: &str) -> bool {
    let Ok(parsed) = key.parse::<AmazonS3ConfigKey>() else {
        return false;
    };
    let name = parsed.as_ref();
    let endpoint_has_credentials = matches!(name, "aws_endpoint" | "aws_endpoint_url_s3")
        && Url::parse(value)
            .is_ok_and(|url| !url.username().is_empty() || url.password().is_some());
    PUBLIC_S3_KEYS.contains(&name) && !endpoint_has_credentials
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
    fn every_public_key_is_a_primary_name_known_to_the_library() {
        // Arrange
        let names = PUBLIC_S3_KEYS;

        // Act
        let reparsed = names.map(|name| {
            name.parse::<AmazonS3ConfigKey>()
                .map(|key| key.as_ref().to_owned())
                .unwrap_or_default()
        });

        // Assert
        assert_eq!(reparsed, names.map(str::to_owned));
    }

    #[test]
    fn only_public_keys_without_credentials_in_the_endpoint_are_allowed() {
        // Arrange
        let cases = [
            ("region", "ru-1"),
            ("aws_allow_http", "true"),
            ("aws_server_side_encryption", "aws:kms"),
            ("endpoint", "http://seaweedfs:8333"),
            ("endpoint", "http://u:SECRET@seaweedfs:8333"),
            ("endpoint", "http://SECRETTOKEN@seaweedfs:8333"),
            ("AWS_SERVER_SIDE_ENCRYPTION", "aws:kms"),
            ("aws_sse_customer_key_base64", "SECRET"),
            ("aws_secret_access_key", "SECRET"),
            ("proxy_url", "http://proxy:3128"),
            ("aws_role_arn", "arn:aws:iam::1:role/x"),
            ("foo", "1"),
        ];

        // Act
        let allowed = cases.map(|(key, value)| is_public_s3_param(key, value));

        // Assert
        assert_eq!(
            allowed,
            [
                true, true, true, true, false, false, false, false, false, false, false, false
            ]
        );
    }

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
