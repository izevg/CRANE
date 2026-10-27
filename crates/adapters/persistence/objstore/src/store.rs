use std::error::Error;
use std::io;
use std::sync::Arc;

use crane_domain::registry::Sha256;
use crane_ports::registry::{BlobError, BlobStore, ByteStream, Digests, Staged, StagingKey};
use futures_util::StreamExt;
use object_store::buffered::BufWriter;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt};
use sha2::Digest;
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

fn backend(e: impl Into<Box<dyn Error + Send + Sync>>) -> BlobError {
    BlobError::Backend(e.into())
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
        let mut hasher = sha2::Sha256::new();
        let mut size = 0u64;
        while let Some(chunk) = body.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(e) => {
                    // Ошибка отката вторична: клиенту важнее причина обрыва, а хвосты
                    // чистит lifecycle-правило на staging/.
                    let _ = writer.abort().await;
                    return Err(BlobError::Body(e));
                }
            };
            hasher.update(&chunk);
            size += chunk.len() as u64;
            if let Err(e) = writer.put(chunk).await {
                let _ = writer.abort().await;
                return Err(backend(e));
            }
        }
        writer.shutdown().await.map_err(backend)?;
        let digests = Digests {
            sha256: Sha256::from(<[u8; 32]>::from(hasher.finalize())),
            size,
        };
        Ok(Staged::new(StagingKey::new(path.to_string()), digests))
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
                Err(object_store::Error::NotFound { .. }) => Err(backend(format!(
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
