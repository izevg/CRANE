//! Успешная публикация: запись, файл, id версии, теги.

use crane_domain::registry::{Asset, Format, FormatMetadata, PackageVersion, VersionState};
use futures_executor::block_on;

use crate::common::{
    HOSTED, Seed, asset_path, caller, coordinate, fixed_now, hello_digests, name, publish_request,
    read, version,
};

use super::HELLO;

#[test]
fn publish_creates_released_record_with_store_digests_and_clock_time() {
    // Arrange
    let seed = Seed::new();
    let hosted = seed.hosted;
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let record = block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    let expected = PackageVersion {
        // id проверяет отдельный тест: здесь он не часть правила.
        id: record.id,
        repository: hosted,
        package: name("foo"),
        version: version("1.0.0"),
        state: VersionState::Released,
        yanked: false,
        deprecated: None,
        published_at: fixed_now(),
        assets: vec![Asset {
            path: asset_path("foo", "1.0.0"),
            content: hello_digests(),
        }],
        metadata: FormatMetadata::new(crate::common::METADATA.to_owned()),
    };
    assert_eq!(record, expected);
}

#[test]
fn published_file_is_fetchable_with_uploaded_bytes() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    let download = block_on(registry.fetch(
        &caller(),
        &coordinate("foo", "1.0.0"),
        &asset_path("foo", "1.0.0"),
    ))
    .unwrap();
    assert_eq!(
        read(download.body),
        b"hello",
        "blob лежит в CAS с теми же байтами"
    );
}

#[test]
fn publish_takes_version_id_from_shared_generator() {
    // Arrange
    // Генератор общий с фикстурой: id версии — следующий после id hosted-репозитория.
    let seed = Seed::new();
    let hosted = seed.hosted;
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let record = block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    assert_eq!(
        record.id.as_uuid().as_u128(),
        hosted.as_uuid().as_u128() + 1
    );
}

#[test]
fn publish_with_tags_sets_them_on_the_new_version() {
    // Arrange
    let registry = Seed::new().registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.tags = vec!["latest".to_owned()];

    // Act
    let record = block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    let package =
        block_on(registry.package(&caller(), Format::Cargo, HOSTED, &name("foo"))).unwrap();
    assert_eq!(package.tags, [("latest".to_owned(), record.id)]);
}
