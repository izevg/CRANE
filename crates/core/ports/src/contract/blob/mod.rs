use std::io;

use bytes::Bytes;
use crane_domain::registry::{Digests, Md5, Sha1, Sha256, Sha512};
use futures_util::{StreamExt, stream};

use crate::registry::{BlobError, BlobStore, ByteStream, Staged, StagingKey};

/// Дайджесты `hello` как константы, чтобы контракт не зависел от хеш-библиотеки.
const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
const HELLO_SHA512: &str = "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043";
const HELLO_SHA1: &str = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
const HELLO_MD5: &str = "5d41402abc4b2a76b9719d911017c592";

/// Проверяет реализацию `BlobStore`; `new` создаёт чистое хранилище для каждого сценария.
pub async fn blob_store_contract<B: BlobStore>(new: impl Fn() -> B) {
    stage_computes_sha256_and_size_over_all_chunks(&new()).await;
    commit_returns_digests_of_content(&new()).await;
    committed_blob_opens_with_exact_bytes(&new()).await;
    commit_of_already_stored_content_is_idempotent(&new()).await;
    commit_of_missing_staged_object_fails(&new()).await;
    commit_after_transfer_is_idempotent(&new()).await;
    open_unknown_is_not_found(&new()).await;
    uncommitted_blob_is_not_readable(&new()).await;
    broken_body_is_reported_as_body_error(&new()).await;
    commit_of_one_staged_object_keeps_other_staged_object_intact(&new()).await;
}

fn world() -> ByteStream {
    body(vec![Ok(Bytes::from_static(b"world"))])
}

fn hello_digests() -> Digests {
    Digests {
        sha256: hello_sha256(),
        sha512: Sha512::from_hex(HELLO_SHA512).expect("константа HELLO_SHA512 — валидный hex"),
        sha1: Sha1::from_hex(HELLO_SHA1).expect("константа HELLO_SHA1 — валидный hex"),
        md5: Md5::from_hex(HELLO_MD5).expect("константа HELLO_MD5 — валидный hex"),
        size: 5,
    }
}

fn body(chunks: Vec<io::Result<Bytes>>) -> ByteStream {
    Box::pin(stream::iter(chunks))
}

fn hello() -> ByteStream {
    body(vec![
        Ok(Bytes::from_static(b"hel")),
        Ok(Bytes::from_static(b"lo")),
    ])
}

fn hello_sha256() -> Sha256 {
    Sha256::from_hex(HELLO_SHA256).expect("константа HELLO_SHA256 — валидный hex")
}

async fn read_all(mut stream: ByteStream) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(chunk) = stream.next().await {
        out.extend_from_slice(&chunk.expect("поток открытого blob не должен падать"));
    }
    out
}

async fn stage_computes_sha256_and_size_over_all_chunks<B: BlobStore>(store: &B) {
    // Arrange
    let input = hello();

    // Act
    let result = store.stage(input).await;

    // Assert
    let staged = result.expect("stage");
    assert_eq!(
        *staged.digests(),
        hello_digests(),
        "stage считает SHA-256, SHA-512, SHA-1 и размер по всем чанкам потока"
    );
}

async fn commit_returns_digests_of_content<B: BlobStore>(store: &B) {
    // Arrange
    let staged = store.stage(hello()).await.expect("stage");

    // Act
    let result = store.commit(staged).await;

    // Assert
    assert_eq!(
        result.expect("commit"),
        hello_digests(),
        "commit возвращает дайджесты содержимого"
    );
}

async fn committed_blob_opens_with_exact_bytes<B: BlobStore>(store: &B) {
    // Arrange
    let staged = store.stage(hello()).await.expect("stage");
    let digests = store.commit(staged).await.expect("commit");

    // Act
    let result = store.open(&digests.sha256).await;

    // Assert
    assert_eq!(
        read_all(result.expect("open закоммиченного blob")).await,
        b"hello",
        "open отдаёт ровно те байты, что были загружены"
    );
}

async fn commit_of_already_stored_content_is_idempotent<B: BlobStore>(store: &B) {
    // Arrange
    let first = store.stage(hello()).await.expect("stage 1");
    let second = store.stage(hello()).await.expect("stage 2");
    let first = store.commit(first).await.expect("первый commit");

    // Act
    let result = store.commit(second).await;

    // Assert
    assert_eq!(
        result.expect("commit идемпотентен: blob уже есть, это не ошибка"),
        first,
        "повторный commit того же содержимого возвращает те же Digests"
    );
}

async fn commit_of_missing_staged_object_fails<B: BlobStore>(store: &B) {
    // Arrange
    let missing = Sha256::from([0xcd; 32]);
    let staged = Staged::new(
        StagingKey::new("missing".to_owned()),
        Digests {
            sha256: missing,
            sha512: Sha512::from([0; 64]),
            sha1: Sha1::from([0; 20]),
            md5: Md5::from([0; 16]),
            size: 1,
        },
    );

    // Act
    let result = store.commit(staged).await;

    // Assert
    assert!(
        matches!(result, Err(BlobError::Backend(_))),
        "commit без staged-объекта и без такого blob — ошибка Backend, а не молчаливая потеря данных"
    );
    let opened = store.open(&missing).await;
    assert!(
        matches!(opened, Err(BlobError::NotFound(_))),
        "неудавшийся commit не создаёт blob: open даёт NotFound"
    );
}

async fn commit_after_transfer_is_idempotent<B: BlobStore>(store: &B) {
    // Arrange
    let staged = store.stage(hello()).await.expect("stage");
    let digests = store.commit(staged).await.expect("первый commit");
    let replay = Staged::new(StagingKey::new("missing".to_owned()), digests);

    // Act
    let result = store.commit(replay).await;

    // Assert
    assert_eq!(
        result.expect(
            "повтор commit после переноса: staged-объекта нет, но blob уже есть — это успех"
        ),
        digests,
        "повторный commit возвращает те же Digests"
    );
}

async fn open_unknown_is_not_found<B: BlobStore>(store: &B) {
    // Arrange
    let unknown = Sha256::from([0xab; 32]);

    // Act
    let result = store.open(&unknown).await;

    // Assert
    assert!(
        matches!(&result, Err(BlobError::NotFound(sha)) if *sha == unknown),
        "open неизвестного дайджеста даёт NotFound с этим дайджестом"
    );
}

async fn uncommitted_blob_is_not_readable<B: BlobStore>(store: &B) {
    // Arrange
    let staged = store.stage(hello()).await.expect("stage");
    let sha256 = staged.digests().sha256;

    // Act
    let result = store.open(&sha256).await;

    // Assert
    assert!(
        matches!(result, Err(BlobError::NotFound(_))),
        "staged-объект без commit недоступен для open"
    );
}

async fn broken_body_is_reported_as_body_error<B: BlobStore>(store: &B) {
    // Arrange
    let broken = body(vec![
        Ok(Bytes::from_static(b"hel")),
        Err(io::Error::other("обрыв")),
    ]);

    // Act
    let result = store.stage(broken).await;

    // Assert
    assert!(
        matches!(&result, Err(BlobError::Body(error)) if error.to_string() == "обрыв"),
        "обрыв входного потока даёт BlobError::Body с исходной ошибкой потока, а не успех и не Backend"
    );
}

// Правило «staged-объекты не мешают друг другу»: пока world ждёт в staging,
// commit hello не должен ни затронуть его, ни подменить его байты; commit world
// в Act проверяет это на втором объекте, а оба blob читаются в Assert.
async fn commit_of_one_staged_object_keeps_other_staged_object_intact<B: BlobStore>(store: &B) {
    // Arrange
    let staged_hello = store.stage(hello()).await.expect("stage hello");
    let staged_world = store.stage(world()).await.expect("stage world");
    let hello_digests = store.commit(staged_hello).await.expect("commit hello");

    // Act
    let result = store.commit(staged_world).await;

    // Assert
    let world_digests = result.expect("commit world");
    let opened_hello = store.open(&hello_digests.sha256).await.expect("open hello");
    let opened_world = store.open(&world_digests.sha256).await.expect("open world");
    assert_eq!(
        (read_all(opened_hello).await, read_all(opened_world).await),
        (b"hello".to_vec(), b"world".to_vec()),
        "одновременно существующие staged-объекты не мешают друг другу: каждый blob отдаёт свои байты"
    );
}
