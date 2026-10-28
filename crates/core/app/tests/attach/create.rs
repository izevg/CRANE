//! Первый файл создаёт версию, следующие прикрепляются к ней.

use crane_domain::registry::{FormatMetadata, VersionState};
use futures_executor::block_on;

use crate::common::{
    METADATA, Seed, asset_path, attach_request, caller, coordinate, hello_digests, read,
};

use super::{FIRST, HELLO, SECOND, paths_of_foo};

#[test]
fn first_file_creates_released_version_with_that_file() {
    // Arrange
    let registry = Seed::new().registry();
    let request = attach_request("foo", "1.0.0", FIRST, &HELLO, Vec::new());

    // Act
    let version = block_on(registry.attach(&caller(), request)).unwrap();

    // Assert
    assert_eq!(version.state, VersionState::Released);
    assert_eq!(
        (version.assets.len(), version.assets[0].content),
        (1, hello_digests())
    );
}

#[test]
fn second_file_is_attached_and_both_files_are_fetchable() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    let registry = seed.registry();
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());

    // Act
    let version = block_on(registry.attach(&caller(), request)).unwrap();

    // Assert
    let fetch = |path: &str| {
        let download = block_on(registry.fetch(&caller(), &coordinate("foo", "1.0.0"), path));
        read(download.unwrap().body)
    };
    assert_eq!(version.assets.len(), 2);
    assert_eq!(
        (fetch(FIRST), fetch(SECOND)),
        (b"first".to_vec(), b"hello".to_vec())
    );
}

#[test]
fn version_metadata_is_written_only_by_the_file_that_created_it() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    let registry = seed.registry();
    let mut request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());
    request.metadata = Ok(FormatMetadata::new(r#"{"deps":["other"]}"#.to_owned()));

    // Act
    let version = block_on(registry.attach(&caller(), request)).unwrap();

    // Assert
    assert_eq!(version.metadata, FormatMetadata::new(METADATA.to_owned()));
}

#[test]
fn files_of_one_version_attach_independently_of_other_versions() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    seed.add_version("foo", "2.0.0", VersionState::Released, b"other");
    let registry = seed.registry();
    let request = attach_request(
        "foo",
        "1.0.0",
        &asset_path("foo", "2.0.0"),
        &HELLO,
        Vec::new(),
    );

    // Act
    let result = block_on(registry.attach(&caller(), request));

    // Assert
    assert!(
        result.is_ok(),
        "путь уникален внутри версии, а не пакета: {result:?}"
    );
    assert_eq!(paths_of_foo(&registry).len(), 3);
}
