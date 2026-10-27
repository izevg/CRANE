use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use crane_domain::Sha256;
use futures_core::Stream;

/// Поток байт тела артефакта; ошибка чтения — `io::Error`.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>;

/// SHA-256 и размер содержимого.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Digests {
    pub sha256: Sha256,
    pub size: u64,
}

/// Непрозрачный ключ временного объекта; смысл знает только породивший его адаптер.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagingKey(String);

impl StagingKey {
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Объект во временной зоне. Не `Clone`: `commit` потребляет значение,
/// поэтому один staged-объект нельзя закоммитить дважды.
#[derive(Debug)]
pub struct Staged {
    key: StagingKey,
    digests: Digests,
}

impl Staged {
    #[must_use]
    pub const fn new(key: StagingKey, digests: Digests) -> Self {
        Self { key, digests }
    }

    #[must_use]
    pub const fn key(&self) -> &StagingKey {
        &self.key
    }

    #[must_use]
    pub const fn digests(&self) -> &Digests {
        &self.digests
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    /// Закоммиченного blob с таким дайджестом нет.
    #[error("blob {0} не найден")]
    NotFound(Sha256),
    /// Входной поток оборвался: это ошибка запроса клиента, а не хранилища.
    #[error("ошибка чтения тела запроса")]
    Body(#[source] std::io::Error),
    #[error("ошибка хранилища blob")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Content-addressable хранилище: ключ blob — его SHA-256.
///
/// Сверка с дайджестом, заявленным клиентом, не дело стора: это правило use case.
pub trait BlobStore: Send + Sync {
    /// Сохраняет поток во временную зону и считает дайджесты.
    /// Обрыв потока даёт `BlobError::Body`; staged-объект при этом не создаётся.
    fn stage(&self, body: ByteStream) -> impl Future<Output = Result<Staged, BlobError>> + Send;

    /// Переносит staged-объект в CAS по его SHA-256.
    /// Идемпотентно: если такой blob уже есть, это успех.
    /// Если staged-объекта нет и такого blob тоже нет, это ошибка (`Backend`):
    /// молча вернуть дайджесты нельзя, версия получила бы ссылку на несуществующий blob.
    fn commit(&self, staged: Staged) -> impl Future<Output = Result<Digests, BlobError>> + Send;

    /// Открывает закоммиченный blob. Staged-объекты недоступны: `NotFound`.
    fn open(&self, sha256: &Sha256) -> impl Future<Output = Result<ByteStream, BlobError>> + Send;
}
