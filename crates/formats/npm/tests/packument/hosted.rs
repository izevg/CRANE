//! Документ hosted-пакета из состояния каталога.

use crane_domain::registry::PackageVersion;
use crane_format_npm::{DocError, View, render_hosted};
use serde_json::{Value, json};

use crate::common::{pkg, stored, version_id};
use crate::{BASE, parse};

const HOSTED_META_1: &str = r#"{"name":"left-pad","version":"1.0.0","dist":{"integrity":"sha512-A","shasum":"ab"},"scripts":{"install":"x"}}"#;
const HOSTED_META_2: &str =
    r#"{"name":"left-pad","version":"2.0.0-rc.1","dist":{"integrity":"sha512-B"}}"#;

fn hosted(
    versions: &[crane_domain::registry::PackageVersion],
    tags: &[(String, crane_domain::registry::VersionId)],
    view: View,
) -> Result<Value, DocError> {
    render_hosted(&pkg("left-pad"), versions, tags, view, BASE).map(|out| parse(&out))
}

/// Две версии с разными метаданными: первая с deprecate и скриптом, вторая prerelease.
fn two_stored_versions() -> [PackageVersion; 2] {
    [
        stored(
            "left-pad",
            "1.0.0",
            1,
            1_704_067_200,
            Some("stop"),
            HOSTED_META_1,
        ),
        stored(
            "left-pad",
            "2.0.0-rc.1",
            2,
            1_704_153_600,
            None,
            HOSTED_META_2,
        ),
    ]
}

#[test]
fn render_hosted_full_builds_document_from_state() {
    // Arrange
    let versions = two_stored_versions();
    let tags = [
        ("latest".to_owned(), version_id(1)),
        ("next".to_owned(), version_id(2)),
        ("ghost".to_owned(), version_id(99)),
    ];

    // Act
    let actual = hosted(&versions, &tags, View::Full);

    // Assert
    assert_eq!(
        actual,
        Ok(json!({
            "_id": "left-pad",
            "_rev": "2-1704153600000",
            "name": "left-pad",
            "dist-tags": {"latest": "1.0.0", "next": "2.0.0-rc.1"},
            "time": {
                "created": "2024-01-01T00:00:00.000Z",
                "modified": "2024-01-02T00:00:00.000Z",
                "1.0.0": "2024-01-01T00:00:00.000Z",
                "2.0.0-rc.1": "2024-01-02T00:00:00.000Z"
            },
            "versions": {
                "1.0.0": {
                    "_id": "left-pad@1.0.0",
                    "name": "left-pad",
                    "version": "1.0.0",
                    "deprecated": "stop",
                    "scripts": {"install": "x"},
                    "dist": {
                        "integrity": "sha512-A",
                        "shasum": "ab",
                        "tarball": "https://h.example/npm/repo/left-pad/-/left-pad-1.0.0.tgz"
                    }
                },
                "2.0.0-rc.1": {
                    "_id": "left-pad@2.0.0-rc.1",
                    "name": "left-pad",
                    "version": "2.0.0-rc.1",
                    "dist": {
                        "integrity": "sha512-B",
                        "tarball": "https://h.example/npm/repo/left-pad/-/left-pad-2.0.0-rc.1.tgz"
                    }
                }
            }
        }))
    );
}

#[test]
fn render_hosted_abbreviated_has_no_rev_time_or_scripts_but_keeps_flags() {
    // Arrange
    let versions = [stored(
        "left-pad",
        "1.0.0",
        1,
        1_704_067_200,
        Some("stop"),
        HOSTED_META_1,
    )];
    let tags = [("latest".to_owned(), version_id(1))];

    // Act
    let actual = hosted(&versions, &tags, View::Abbreviated);

    // Assert
    assert_eq!(
        actual,
        Ok(json!({
            "name": "left-pad",
            "modified": "2024-01-01T00:00:00.000Z",
            "dist-tags": {"latest": "1.0.0"},
            "versions": {"1.0.0": {
                "name": "left-pad",
                "version": "1.0.0",
                "deprecated": "stop",
                "hasInstallScript": true,
                "dist": {
                    "integrity": "sha512-A",
                    "shasum": "ab",
                    "tarball": "https://h.example/npm/repo/left-pad/-/left-pad-1.0.0.tgz"
                }
            }}
        }))
    );
}

#[test]
fn render_hosted_without_versions_is_not_a_document() {
    // Arrange
    let versions = [];

    // Act
    let actual = render_hosted(&pkg("left-pad"), &versions, &[], View::Full, BASE);

    // Assert
    assert_eq!(actual, Err(DocError::NoVersions));
}
