mod common;

use std::io;

use bytes::Bytes;
use common::{TempDir, body, chunks, count_files, run, store_in};
use crane_domain::registry::Sha256;
use crane_ports::registry::{BlobError, BlobStore, Staged, StagingKey};
use futures_util::StreamExt;

const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

#[test]
fn staging_is_empty_after_commit() {
    // Arrange
    let dir = TempDir::new();
    let store = store_in(dir.path());

    // Act
    run(async {
        let staged = store.stage(chunks(&[b"hel", b"lo"])).await.expect("stage");
        assert_eq!(
            count_files(&dir.path().join("staging")),
            1,
            "до commit объект в staging"
        );
        store.commit(staged).await.expect("commit");
    });

    // Assert
    assert_eq!(count_files(&dir.path().join("staging")), 0);
}

#[test]
fn blob_is_stored_under_fan_out_path() {
    // Arrange
    let dir = TempDir::new();
    let store = store_in(dir.path());

    // Act
    run(async {
        let staged = store.stage(chunks(&[b"hello"])).await.expect("stage");
        store.commit(staged).await.expect("commit");
    });

    // Assert
    let path = dir
        .path()
        .join(format!("blobs/sha256/2c/f2/{HELLO_SHA256}"));
    assert_eq!(
        std::fs::read(path).expect("blob лежит по раскладке"),
        b"hello"
    );
}

#[test]
fn stage_reports_size_and_digest_of_whole_body() {
    // Arrange
    let dir = TempDir::new();
    let store = store_in(dir.path());

    // Act
    let staged = run(store.stage(chunks(&[b"he", b"l", b"lo"]))).expect("stage");

    // Assert
    let digests = staged.digests();
    assert_eq!(digests.size, 5);
    assert_eq!(digests.sha256.to_string(), HELLO_SHA256);
}

#[test]
fn committing_duplicate_content_is_idempotent_and_leaves_staging_empty() {
    // Arrange
    let dir = TempDir::new();
    let store = store_in(dir.path());

    // Act
    let (first, second) = run(async {
        let a = store.stage(chunks(&[b"hello"])).await.expect("stage a");
        let b = store.stage(chunks(&[b"hello"])).await.expect("stage b");
        let first = store.commit(a).await.expect("commit a");
        let second = store.commit(b).await.expect("commit b");
        (first, second)
    });

    // Assert
    assert_eq!(first, second);
    assert_eq!(count_files(&dir.path().join("staging")), 0);
    assert_eq!(count_files(&dir.path().join("blobs")), 1);
}

#[test]
fn broken_body_leaves_nothing_in_staging() {
    // Arrange: чанк больше буфера BufWriter (10 МиБ) запускает multipart-загрузку,
    // которую после обрыва нужно откатить.
    let dir = TempDir::new();
    let store = store_in(dir.path());
    let big = Bytes::from(vec![7u8; 11 * 1024 * 1024]);
    let broken = body(vec![Ok(big), Err(io::Error::other("обрыв"))]);

    // Act
    let result = run(store.stage(broken));

    // Assert
    assert!(matches!(result, Err(BlobError::Body(_))));
    assert_eq!(count_files(&dir.path().join("staging")), 0);
}

#[test]
fn large_body_round_trips_through_multipart_path() {
    // Arrange
    let dir = TempDir::new();
    let store = store_in(dir.path());
    let data = vec![42u8; 11 * 1024 * 1024 + 5];

    // Act
    let (digests, read) = run(async {
        let staged = store
            .stage(body(vec![Ok(Bytes::from(data.clone()))]))
            .await
            .expect("stage");
        let digests = store.commit(staged).await.expect("commit");
        let mut stream = store.open(&digests.sha256).await.expect("open");
        let mut read = Vec::new();
        while let Some(chunk) = stream.next().await {
            read.extend_from_slice(&chunk.expect("chunk"));
        }
        (digests, read)
    });

    // Assert
    assert_eq!(digests.size, data.len() as u64);
    assert_eq!(read, data);
}

#[test]
fn staged_object_is_not_visible_through_open() {
    // Arrange
    let dir = TempDir::new();
    let store = store_in(dir.path());
    let sha = Sha256::from_hex(HELLO_SHA256).expect("hex");

    // Act
    let result = run(async {
        store.stage(chunks(&[b"hello"])).await.expect("stage");
        store.open(&sha).await
    });

    // Assert
    assert!(matches!(result, Err(BlobError::NotFound(s)) if s == sha));
}

#[test]
fn recommitting_already_committed_staging_key_succeeds() {
    // Arrange: повтор commit после сбоя ответа; staging-объекта уже нет, blob на месте
    let dir = TempDir::new();
    let store = store_in(dir.path());
    let staged = run(store.stage(chunks(&[b"hello"]))).expect("stage");
    let key = staged.key().as_str().to_owned();
    let digests = run(store.commit(staged)).expect("первый commit");
    let again = Staged::new(StagingKey::new(key), digests);

    // Act
    let result = run(store.commit(again));

    // Assert
    assert_eq!(result.expect("повторный commit"), digests);
}
