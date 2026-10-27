//! Сбои тела и хранилища при публикации ничего не оставляют.

use std::io;

use bytes::Bytes;
use crane_app::registry::{Publish, RegistryError, Upload};
use crane_domain::registry::FormatMetadata;
use futures_executor::block_on;
use futures_util::stream;

use crate::common::faults::{RecordingBlobs, commit_log, committed};
use crate::common::{Seed, asset_path, caller, coordinate, publish_request};

use super::{HELLO, listed_foo};

/// Запрос, тело которого обрывается ошибкой ввода-вывода.
fn broken_body_request() -> Publish {
    let items = vec![
        Ok(Bytes::from_static(b"ab")),
        Err(io::Error::other("обрыв")),
    ];
    Publish {
        coordinate: coordinate("foo", "1.0.0"),
        file: Upload {
            path: asset_path("foo", "1.0.0"),
            body: Box::pin(stream::iter(items)),
            declared: Vec::new(),
        },
        metadata: FormatMetadata::new(crate::common::METADATA.to_owned()),
        tags: Vec::new(),
    }
}

#[test]
fn publish_with_broken_body_reports_body_error_and_publishes_nothing() {
    // Arrange
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = broken_body_request();

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::Body(_)),
        "обрыв тела — ошибка клиента: {error:?}"
    );
    assert!(
        committed(&log).is_empty(),
        "оборванное тело не попадает в CAS"
    );
    assert!(listed_foo(&registry).is_empty(), "версия не опубликована");
}

#[test]
fn publish_reports_backend_error_and_records_nothing_when_blob_commit_fails() {
    // Arrange
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::failing_commit(blobs, commit_log()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::Backend(_)), "{error:?}");
    assert!(
        listed_foo(&registry).is_empty(),
        "запись каталога не должна ссылаться на незакоммиченный blob"
    );
}
