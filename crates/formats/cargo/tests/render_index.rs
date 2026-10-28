#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use std::time::SystemTime;

use bytes::Bytes;
use crane_domain::registry::{
    Asset, Digests, Ecosystem, FormatMetadata, Md5, PackageVersion, RepositoryId, Sha1, Sha256,
    Sha512, VersionId, VersionState,
};
use crane_format_cargo::{CARGO, IndexError, asset_path, parse_publish, render_index};
use serde_json::{Value, json};
use uuid::Uuid;

const OWN_INDEX: &str = "sparse+https://crane.example/cargo/internal/index/";

fn publish_body(name: &str, vers: &str) -> Bytes {
    let json = json!({
        "name": name,
        "vers": vers,
        "deps": [{
            "name": "serde_json", "version_req": "^1", "features": ["std"],
            "optional": true, "default_features": false, "target": "cfg(unix)",
            "kind": null, "registry": null, "explicit_name_in_toml": "sj"
        }],
        "features": { "json": ["dep:serde_json"] },
        "links": "native",
        "rust_version": "1.74",
        "description": "не должно попасть в индекс"
    })
    .to_string();
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(json.len()).unwrap().to_le_bytes());
    out.extend_from_slice(json.as_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.push(b'x');
    Bytes::from(out)
}

fn digests(byte: u8) -> Digests {
    Digests {
        sha256: Sha256::from([byte; 32]),
        sha512: Sha512::from([byte; 64]),
        sha1: Sha1::from([byte; 20]),
        md5: Md5::from([byte; 16]),
        size: 1,
    }
}

fn version(name: &str, vers: &str, yanked: bool, sha_byte: u8) -> PackageVersion {
    let publish = parse_publish(publish_body(name, vers)).unwrap();
    let metadata = publish.index_metadata(OWN_INDEX).unwrap();
    PackageVersion {
        id: VersionId::new(Uuid::nil()),
        repository: RepositoryId::new(Uuid::nil()),
        assets: vec![Asset {
            path: asset_path(&publish.name, &publish.version),
            content: digests(sha_byte),
        }],
        package: publish.name,
        version: publish.version,
        state: VersionState::Released,
        yanked,
        deprecated: None,
        published_at: SystemTime::UNIX_EPOCH,
        metadata,
    }
}

fn lines(index: &str) -> Vec<Value> {
    index
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn each_version_is_one_line_terminated_by_newline_in_input_order() {
    // Arrange
    let versions = [
        version("Foo-Bar", "1.0.0", false, 0x11),
        version("Foo-Bar", "1.1.0", false, 0x22),
    ];

    // Act
    let index = render_index(&versions).unwrap();

    // Assert
    assert!(index.ends_with('\n'));
    assert_eq!(index.matches('\n').count(), 2);
    let parsed = lines(&index);
    assert_eq!(parsed[0]["vers"], "1.0.0");
    assert_eq!(parsed[1]["vers"], "1.1.0");
}

#[test]
fn line_has_exact_cargo_index_shape() {
    // Arrange
    let versions = [version("Foo-Bar", "1.0.0", false, 0xab)];

    // Act
    let index = render_index(&versions).unwrap();

    // Assert
    let expected = json!({
        "name": "Foo-Bar",
        "vers": "1.0.0",
        "deps": [{
            "name": "sj", "req": "^1", "features": ["std"], "optional": true,
            "default_features": false, "target": "cfg(unix)", "kind": "normal",
            "registry": null, "package": "serde_json"
        }],
        "cksum": "ab".repeat(32),
        "features": { "json": ["dep:serde_json"] },
        "yanked": false,
        "links": "native",
        "rust_version": "1.74"
    });
    assert_eq!(lines(&index), [expected]);
}

#[test]
fn yanked_flag_is_reflected_per_version() {
    // Arrange
    let versions = [
        version("foo", "1.0.0", true, 1),
        version("foo", "1.1.0", false, 2),
    ];

    // Act
    let index = render_index(&versions).unwrap();

    // Assert
    let parsed = lines(&index);
    assert_eq!(parsed[0]["yanked"], true);
    assert_eq!(parsed[1]["yanked"], false);
}

#[test]
fn checksum_comes_from_the_asset_of_that_very_version() {
    // Arrange
    let mut v = version("foo", "1.0.0", false, 0x01);
    v.assets.insert(
        0,
        Asset {
            path: "other.txt".to_owned(),
            content: digests(0xee),
        },
    );

    // Act
    let index = render_index(&[v]).unwrap();

    // Assert
    assert_eq!(lines(&index)[0]["cksum"], "01".repeat(32));
}

#[test]
fn empty_input_gives_empty_index() {
    // Arrange

    // Act
    let index = render_index(&[]).unwrap();

    // Assert
    assert_eq!(index, "");
}

#[test]
fn missing_crate_asset_is_reported_with_package_and_version() {
    // Arrange
    let mut v = version("foo", "1.0.0", false, 1);
    v.assets.clear();

    // Act
    let result = render_index(&[v]);

    // Assert
    match result {
        Err(IndexError::MissingCrateFile { package, version }) => {
            assert_eq!((package.as_str(), version.as_str()), ("foo", "1.0.0"));
        }
        other => panic!("ожидалась MissingCrateFile, получено {other:?}"),
    }
}

#[test]
fn garbage_metadata_is_reported() {
    // Arrange
    let mut v = version("foo", "1.0.0", false, 1);
    v.metadata = FormatMetadata::new("не json".to_owned());

    // Act
    let result = render_index(&[v]);

    // Assert
    assert!(matches!(result, Err(IndexError::Metadata(_))), "{result:?}");
}

#[test]
fn name_and_version_are_written_as_published() {
    // Arrange
    let name = CARGO.parse_name("Foo_Bar").unwrap();
    let vers = CARGO.parse_version("1.0.0+meta").unwrap();
    let mut v = version("Foo_Bar", "1.0.0+meta", false, 3);
    v.package = name;
    v.version = vers;

    // Act
    let index = render_index(&[v]).unwrap();

    // Assert
    let line = &lines(&index)[0];
    assert_eq!(
        (&line["name"], &line["vers"]),
        (&json!("Foo_Bar"), &json!("1.0.0+meta"))
    );
}
