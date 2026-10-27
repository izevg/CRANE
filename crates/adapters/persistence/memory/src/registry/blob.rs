use std::collections::HashMap;
use std::future::ready;
use std::io;
use std::sync::{Mutex, MutexGuard, PoisonError};

use bytes::{Bytes, BytesMut};
use crane_domain::registry::Sha256;
use crane_ports::registry::{BlobError, BlobStore, ByteStream, Digests, Staged, StagingKey};
use futures_util::{StreamExt, stream};
use sha2::Digest;

/// Blob-хранилище в памяти.
#[derive(Debug, Default)]
pub struct MemoryBlobStore {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    next_key: u64,
    staging: HashMap<String, Bytes>,
    blobs: HashMap<Sha256, Bytes>,
}

impl MemoryBlobStore {
    // Замок берётся только в синхронных функциях: guard не должен жить через await.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn put_staged(&self, data: Bytes) -> String {
        let mut state = self.lock();
        state.next_key += 1;
        let key = state.next_key.to_string();
        state.staging.insert(key.clone(), data);
        key
    }

    fn promote(&self, key: &StagingKey, sha256: Sha256) -> Result<(), BlobError> {
        let mut state = self.lock();
        match state.staging.remove(key.as_str()) {
            Some(data) => {
                state.blobs.entry(sha256).or_insert(data);
                Ok(())
            }
            // Дубликат уже закоммичен раньше: идемпотентность сохраняется.
            None if state.blobs.contains_key(&sha256) => Ok(()),
            // Иначе use case запишет версию со ссылкой на несуществующий blob.
            None => Err(BlobError::Backend(
                format!("staged-объект {} не найден", key.as_str()).into(),
            )),
        }
    }

    fn get(&self, sha256: &Sha256) -> Option<Bytes> {
        self.lock().blobs.get(sha256).cloned()
    }
}

impl BlobStore for MemoryBlobStore {
    async fn stage(&self, mut body: ByteStream) -> Result<Staged, BlobError> {
        let mut hasher = sha2::Sha256::new();
        let mut data = BytesMut::new();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(BlobError::Body)?;
            hasher.update(&chunk);
            data.extend_from_slice(&chunk);
        }
        let digests = Digests {
            sha256: Sha256::from(<[u8; 32]>::from(hasher.finalize())),
            size: data.len() as u64,
        };
        let key = self.put_staged(data.freeze());
        Ok(Staged::new(StagingKey::new(key), digests))
    }

    // `async fn` без `.await` не проходит `clippy::unused_async`, поэтому готовые футуры.
    fn commit(&self, staged: Staged) -> impl Future<Output = Result<Digests, BlobError>> + Send {
        let committed = self
            .promote(staged.key(), staged.digests().sha256)
            .map(|()| *staged.digests());
        ready(committed)
    }

    fn open(&self, sha256: &Sha256) -> impl Future<Output = Result<ByteStream, BlobError>> + Send {
        let opened = self
            .get(sha256)
            .ok_or(BlobError::NotFound(*sha256))
            .map(|data| -> ByteStream { Box::pin(stream::once(ready(Ok::<_, io::Error>(data)))) });
        ready(opened)
    }
}
