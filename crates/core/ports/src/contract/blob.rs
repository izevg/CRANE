use std::io;

use bytes::Bytes;
use crane_domain::registry::Sha256;
use futures_util::{StreamExt, stream};

use crate::registry::{BlobError, BlobStore, ByteStream, Digests, Staged, StagingKey};

/// SHA-256 от `hello`; константа, чтобы не тянуть хеш-библиотеку в порты.
const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

/// Проверяет реализацию `BlobStore`; `new` создаёт чистое хранилище для каждого сценария.
pub async fn blob_store_contract<B: BlobStore>(new: impl Fn() -> B) {
    staged_blob_is_readable_after_commit(&new()).await;
    commit_is_idempotent(&new()).await;
    open_unknown_is_not_found(&new()).await;
    uncommitted_blob_is_not_readable(&new()).await;
    broken_body_is_reported_as_body_error(&new()).await;
    commit_of_missing_staged_object_fails(&new()).await;
    staged_objects_are_independent(&new()).await;
    commit_after_transfer_is_idempotent(&new()).await;
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

async fn staged_blob_is_readable_after_commit<B: BlobStore>(store: &B) {
    let staged = store.stage(hello()).await.expect("stage");
    assert_eq!(
        (staged.digests().sha256, staged.digests().size),
        (hello_sha256(), 5),
        "stage считает SHA-256 и размер по всем чанкам потока"
    );

    let digests = store.commit(staged).await.expect("commit");
    assert_eq!(
        digests.sha256,
        hello_sha256(),
        "commit возвращает дайджесты содержимого"
    );
    let opened = store
        .open(&digests.sha256)
        .await
        .expect("open закоммиченного blob");

    assert_eq!(
        read_all(opened).await,
        b"hello",
        "open отдаёт ровно те байты, что были загружены"
    );
}

async fn commit_is_idempotent<B: BlobStore>(store: &B) {
    let first = store.stage(hello()).await.expect("stage 1");
    let second = store.stage(hello()).await.expect("stage 2");

    let first = store.commit(first).await.expect("первый commit");
    let second = store.commit(second).await;

    let second = second.expect("commit идемпотентен: blob уже есть, это не ошибка");
    assert_eq!(
        first, second,
        "повторный commit того же содержимого возвращает те же Digests"
    );
}

async fn open_unknown_is_not_found<B: BlobStore>(store: &B) {
    let unknown = Sha256::from([0xab; 32]);

    let result = store.open(&unknown).await;

    assert!(
        matches!(&result, Err(BlobError::NotFound(sha)) if *sha == unknown),
        "open неизвестного дайджеста даёт NotFound с этим дайджестом"
    );
}

async fn uncommitted_blob_is_not_readable<B: BlobStore>(store: &B) {
    let staged = store.stage(hello()).await.expect("stage");

    let result = store.open(&staged.digests().sha256).await;

    assert!(
        matches!(result, Err(BlobError::NotFound(_))),
        "staged-объект без commit недоступен для open"
    );
}

async fn broken_body_is_reported_as_body_error<B: BlobStore>(store: &B) {
    let broken = body(vec![
        Ok(Bytes::from_static(b"hel")),
        Err(io::Error::other("обрыв")),
    ]);

    let result = store.stage(broken).await;

    assert!(
        matches!(result, Err(BlobError::Body(_))),
        "обрыв входного потока даёт BlobError::Body, а не успех и не Backend"
    );
}

async fn commit_of_missing_staged_object_fails<B: BlobStore>(store: &B) {
    let missing = Sha256::from([0xcd; 32]);
    let staged = Staged::new(
        StagingKey::new("missing".to_owned()),
        Digests {
            sha256: missing,
            size: 1,
        },
    );

    let result = store.commit(staged).await;

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

async fn staged_objects_are_independent<B: BlobStore>(store: &B) {
    let hello = store.stage(hello()).await.expect("stage hello");
    let world = store
        .stage(body(vec![Ok(Bytes::from_static(b"world"))]))
        .await
        .expect("stage world");

    let hello = store.commit(hello).await.expect("commit hello");
    let world = store.commit(world).await.expect("commit world");

    let opened_hello = store.open(&hello.sha256).await.expect("open hello");
    let opened_world = store.open(&world.sha256).await.expect("open world");
    assert_eq!(
        (read_all(opened_hello).await, read_all(opened_world).await),
        (b"hello".to_vec(), b"world".to_vec()),
        "одновременно существующие staged-объекты не мешают друг другу: каждый blob отдаёт свои байты"
    );
}

async fn commit_after_transfer_is_idempotent<B: BlobStore>(store: &B) {
    let staged = store.stage(hello()).await.expect("stage");
    let digests = store.commit(staged).await.expect("первый commit");
    let replay = Staged::new(StagingKey::new("missing".to_owned()), digests);

    let result = store.commit(replay).await;

    let again = result
        .expect("повтор commit после переноса: staged-объекта нет, но blob уже есть — это успех");
    assert_eq!(again, digests, "повторный commit возвращает те же Digests");
}
