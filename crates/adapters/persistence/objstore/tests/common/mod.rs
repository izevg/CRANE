//! Общие помощники интеграционных тестов: временный каталог и ручной runtime.
#![allow(
    clippy::expect_used,
    dead_code,
    unreachable_pub,
    reason = "помощники тестов: падение — ошибка окружения; каждый тестовый крейт берёт своё подмножество"
)]

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use crane_domain::registry::Sha256;
use crane_objstore::ObjectBlobStore;
use crane_ports::registry::{BlobStore, ByteStream};
use futures_util::{StreamExt, stream};
use url::Url;
use uuid::Uuid;

/// Временный каталог, удаляемый при выходе из теста.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("crane-objstore-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("временный каталог создаётся");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `LocalFileSystem` уходит в `spawn_blocking`, поэтому нужен tokio runtime.
/// Собираем вручную: проект не использует `#[tokio::test]` (процедурный макрос).
pub fn run<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime собирается")
        .block_on(future)
}

pub fn file_url(dir: &Path) -> Url {
    Url::from_directory_path(dir).expect("абсолютный путь")
}

pub fn store_in(dir: &Path) -> ObjectBlobStore {
    ObjectBlobStore::from_url(&file_url(dir), None).expect("file:// хранилище создаётся")
}

pub fn body(chunks: Vec<io::Result<Bytes>>) -> ByteStream {
    Box::pin(stream::iter(chunks))
}

pub fn chunks(parts: &[&'static [u8]]) -> ByteStream {
    body(parts.iter().map(|p| Ok(Bytes::from_static(p))).collect())
}

/// Число файлов под `dir`; отсутствующий каталог — это ноль.
pub fn count_files(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .map(|e| e.expect("запись каталога читается").path())
        .map(|p| if p.is_dir() { count_files(&p) } else { 1 })
        .sum()
}

/// Читает blob целиком через `open`.
pub async fn read_all(store: &ObjectBlobStore, sha256: &Sha256) -> Vec<u8> {
    let mut stream = store.open(sha256).await.expect("open");
    let mut read = Vec::new();
    while let Some(chunk) = stream.next().await {
        read.extend_from_slice(&chunk.expect("chunk"));
    }
    read
}
