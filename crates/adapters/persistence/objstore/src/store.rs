use std::error::Error;
use std::io;
use std::sync::Arc;

use crane_domain::registry::Sha256;
use crane_ports::registry::{
    BlobError, BlobStore, ByteStream, Digester, Digests, Staged, StagingKey,
};
use futures_util::StreamExt;
use object_store::buffered::BufWriter;
use object_store::client::{HttpError, HttpErrorKind};
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

/// Blob-хранилище на `object_store`: локальная ФС или S3, выбирается URL (см. [`Self::from_url`]).
#[derive(Debug)]
pub struct ObjectBlobStore {
    store: Arc<dyn ObjectStore>,
}

impl ObjectBlobStore {
    pub(crate) const fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }
}

/// Единственная воронка ошибок хранилища: решает, «не отвечает» это или сбой/баг.
///
/// Принимает `object_store::Error` и `io::Error` (его отдаёт `BufWriter`, внутри та же цепочка).
fn backend(error: impl Error + Send + Sync + 'static) -> BlobError {
    if unreachable(&error) {
        BlobError::Unreachable(Box::new(error))
    } else {
        BlobError::Backend(Box::new(error))
    }
}

/// Сбой не из `object_store` (нарушенный инвариант): повтор не поможет.
fn invariant(what: String) -> BlobError {
    BlobError::Backend(what.into())
}

/// Не удалось соединиться, вышло время или S3 ответил 5xx: повтор позже имеет смысл.
///
/// Типы `RetryError` и `RequestError` в `object_store` не публичны, поэтому статус ответа
/// читается из текста звена цепочки (формат `RequestError::Status`); `HttpError` публичен
/// и разбирается по типу. Обе ветки закреплены тестами на настоящем HTTP-сервере.
fn unreachable(error: &(dyn Error + 'static)) -> bool {
    let mut link: Option<&(dyn Error + 'static)> = Some(error);
    while let Some(current) = link {
        if let Some(http) = current.downcast_ref::<HttpError>()
            && matches!(
                http.kind(),
                HttpErrorKind::Connect | HttpErrorKind::Timeout | HttpErrorKind::Request
            )
        {
            return true;
        }
        if current.to_string().starts_with(SERVER_STATUS) {
            return server_status_is_5xx(&current.to_string());
        }
        link = current.source();
    }
    false
}

/// Начало текста `RequestError::Status`/`Response` в `object_store` 0.14.
const SERVER_STATUS: &str = "Server returned non-2xx status code: ";

fn server_status_is_5xx(message: &str) -> bool {
    message
        .strip_prefix(SERVER_STATUS)
        .is_some_and(|rest| rest.starts_with('5'))
}

/// Двухуровневая развёртка по префиксу хэша не даёт каталогу на ФС разрастаться до миллионов записей.
fn blob_path(sha256: &Sha256) -> Path {
    let hex = sha256.to_string();
    Path::from(format!("blobs/sha256/{}/{}/{hex}", &hex[0..2], &hex[2..4]))
}

impl BlobStore for ObjectBlobStore {
    async fn stage(&self, mut body: ByteStream) -> Result<Staged, BlobError> {
        let path = Path::from(format!("staging/{}", Uuid::new_v4()));
        let mut writer = BufWriter::new(Arc::clone(&self.store), path.clone());
        let mut digester = Digester::default();
        // Отдельный блок, чтобы откат был в одной точке. shutdown сюда не входит:
        // abort после него паникует («Already shut down»).
        let streamed: Result<(), BlobError> = async {
            while let Some(chunk) = body.next().await {
                let chunk = chunk.map_err(BlobError::Body)?;
                digester.update(&chunk);
                writer.put(chunk).await.map_err(backend)?;
            }
            Ok(())
        }
        .await;
        if let Err(e) = streamed {
            // Ошибка отката вторична: клиенту важнее причина сбоя, а хвосты
            // чистит lifecycle-правило на staging/.
            let _ = writer.abort().await;
            return Err(e);
        }
        writer.shutdown().await.map_err(backend)?;
        Ok(Staged::new(
            StagingKey::new(path.to_string()),
            digester.finish(),
        ))
    }

    async fn commit(&self, staged: Staged) -> Result<Digests, BlobError> {
        let target = blob_path(&staged.digests().sha256);
        let source = Path::from(staged.key().as_str());
        // Перезапись безвредна: ключ — хэш содержимого, байты те же. rename заодно убирает staging.
        match self.store.rename(&source, &target).await {
            Ok(()) => Ok(*staged.digests()),
            // Staging уже нет: дубликат мог быть закоммичен раньше. Это успех только если blob на месте,
            // иначе версия получила бы ссылку на несуществующий blob.
            Err(object_store::Error::NotFound { .. }) => match self.store.head(&target).await {
                Ok(_) => Ok(*staged.digests()),
                Err(object_store::Error::NotFound { .. }) => Err(invariant(format!(
                    "staged-объект {} не найден",
                    staged.key().as_str()
                ))),
                Err(e) => Err(backend(e)),
            },
            Err(e) => Err(backend(e)),
        }
    }

    async fn open(&self, sha256: &Sha256) -> Result<ByteStream, BlobError> {
        match self.store.get(&blob_path(sha256)).await {
            Ok(result) => Ok(Box::pin(
                result
                    .into_stream()
                    .map(|chunk| chunk.map_err(io::Error::other)),
            )),
            Err(object_store::Error::NotFound { .. }) => Err(BlobError::NotFound(*sha256)),
            Err(e) => Err(backend(e)),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    use object_store::ClientConfigKey;
    use object_store::RetryConfig;
    use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey};

    use super::*;

    /// Однократный HTTP-сервер на loopback: читает запрос и отвечает статусом.
    fn serve_status(status: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer).unwrap();
            let reply =
                format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            stream.write_all(reply.as_bytes()).unwrap();
        });
        format!("http://{address}")
    }

    /// Сервер принимает соединение и обрывает его без ответа: `HttpErrorKind::Request`.
    fn serve_hangup() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer).unwrap();
        });
        format!("http://{address}")
    }

    /// Адрес, на котором никто не слушает: соединение отклоняется.
    fn closed_port() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    }

    fn store_at(endpoint: &str) -> ObjectBlobStore {
        let s3 = AmazonS3Builder::new()
            .with_endpoint(endpoint)
            .with_allow_http(true)
            .with_bucket_name("crane")
            .with_region("us-east-1")
            .with_access_key_id("test")
            .with_secret_access_key("test")
            // Тест не зависит от HTTP_PROXY окружения: loopback идёт напрямую, иначе прокси
            // подменил бы обрыв соединения своим 503 и ветки Connect/Request остались бы непроверенными.
            .with_config(
                AmazonS3ConfigKey::Client(ClientConfigKey::ProxyUrl),
                "http://127.0.0.1:9",
            )
            .with_config(
                AmazonS3ConfigKey::Client(ClientConfigKey::ProxyExcludes),
                "127.0.0.1",
            )
            // Без повторов: тест проверяет классификацию, а не бюджет ретраев.
            .with_retry(RetryConfig {
                max_retries: 0,
                retry_timeout: Duration::from_secs(1),
                ..RetryConfig::default()
            })
            .build()
            .unwrap();
        ObjectBlobStore::new(Arc::new(s3))
    }

    #[test]
    fn open_reports_unreachable_only_for_connection_failures_and_5xx() {
        // Arrange
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let endpoints = [
            closed_port(),
            serve_hangup(),
            serve_status("503 Service Unavailable"),
            serve_status("500 Internal Server Error"),
            serve_status("403 Forbidden"),
            serve_status("400 Bad Request"),
        ];
        let digest = Sha256::from([7; 32]);

        // Act
        let actual = endpoints.map(|endpoint| {
            let result = runtime.block_on(store_at(&endpoint).open(&digest));
            matches!(result, Err(BlobError::Unreachable(_)))
        });

        // Assert
        assert_eq!(actual, [true, true, true, true, false, false]);
    }

    #[test]
    fn open_of_a_missing_object_stays_not_found() {
        // Arrange
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let store = store_at(&serve_status("404 Not Found"));
        let digest = Sha256::from([7; 32]);

        // Act
        let result = runtime.block_on(store.open(&digest));

        // Assert
        assert!(matches!(result, Err(BlobError::NotFound(_))));
    }
}
