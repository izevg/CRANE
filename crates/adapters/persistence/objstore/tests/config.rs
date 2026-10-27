mod common;

use common::{TempDir, file_url};
use crane_objstore::ObjectBlobStore;
use object_store::Error;
use url::Url;

#[allow(clippy::expect_used, reason = "помощник теста")]
fn url(s: &str) -> Url {
    Url::parse(s).expect("валидный URL")
}

#[test]
fn file_url_creates_missing_directory() {
    // Arrange
    let root = TempDir::new();
    let target = root.path().join("nested/blobs");

    // Act
    let result = ObjectBlobStore::from_url(&file_url(&target));

    // Assert
    assert!(result.is_ok());
    assert!(target.is_dir());
}

#[test]
fn file_url_with_query_is_rejected() {
    // Arrange
    let root = TempDir::new();
    let mut input = file_url(root.path());
    input.set_query(Some("x=1"));

    // Act
    let result = ObjectBlobStore::from_url(&input);

    // Assert
    assert!(matches!(result, Err(Error::NotSupported { .. })));
}

#[test]
fn s3_url_with_endpoint_builds_without_network() {
    // Arrange
    let input = url("s3://blobs?endpoint=http://127.0.0.1:1&aws_allow_http=true");

    // Act
    let result = ObjectBlobStore::from_url(&input);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn s3_url_with_path_prefix_builds() {
    // Arrange
    let input = url("s3://blobs/team/cas?endpoint=http://127.0.0.1:1&aws_allow_http=true");

    // Act
    let result = ObjectBlobStore::from_url(&input);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn s3_url_with_unknown_key_is_rejected() {
    // Arrange
    let input = url("s3://blobs?no_such_option=1");

    // Act
    let result = ObjectBlobStore::from_url(&input);

    // Assert
    assert!(matches!(result, Err(Error::UnknownConfigurationKey { .. })));
}

#[test]
fn unsupported_scheme_is_rejected() {
    // Arrange
    let input = url("gcs://bucket/path");

    // Act
    let result = ObjectBlobStore::from_url(&input);

    // Assert
    assert!(matches!(result, Err(Error::NotSupported { .. })));
}
