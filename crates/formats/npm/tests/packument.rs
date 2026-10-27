#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::{pkg, stored, version_id};
use crane_format_npm::{DocError, TarballBase, View, render_hosted, render_upstream};
use serde_json::{Value, json};

const BASE: TarballBase<'static> = TarballBase("https://h.example/npm/repo/");

fn parse(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

fn upstream_doc() -> Value {
    json!({
        "_id": "@scope/pkg",
        "name": "@scope/pkg",
        "readme": "очень длинный readme",
        "weird\"key": [1, {"a": null}],
        "dist-tags": {"latest": "1.0.0"},
        "time": {"modified": "2024-01-02T03:04:05.000Z", "1.0.0": "2024-01-01T00:00:00.000Z"},
        "versions": {
            "1.0.0": {
                "name": "@scope/pkg",
                "version": "1.0.0",
                "description": "d",
                "scripts": {"postinstall": "node x", "test": "t"},
                "dependencies": {"a": "1"},
                "dist": {
                    "tarball": "https://evil.example/x.tgz",
                    "integrity": "sha512-AAAA",
                    "shasum": "ab",
                    "fileCount": 3
                }
            },
            "not a version": {"name": "@scope/pkg", "dist": {"tarball": "https://evil.example/y.tgz"}},
            "1.0.1": {"name": "@other/pkg", "version": "1.0.1", "dist": {"tarball": "https://evil.example/z.tgz"}},
            "1.0.2": {"name": "@scope/pkg", "version": "1.0.2"},
            "1.0.3": {"name": "@scope/pkg", "version": "1.0.3", "dist": "oops"}
        }
    })
}

fn render(doc: &Value, view: View) -> Result<Value, DocError> {
    let raw = serde_json::to_vec(doc).unwrap();
    render_upstream(&raw, &pkg("@scope/pkg"), view, BASE).map(|out| parse(&out))
}

#[test]
fn render_upstream_full_rewrites_tarball_to_own_base_and_keeps_other_fields() {
    // Arrange
    let doc = upstream_doc();

    // Act
    let actual = render(&doc, View::Full);

    // Assert
    let mut expected = doc.clone();
    expected["versions"] = json!({
        "1.0.0": {
            "name": "@scope/pkg",
            "version": "1.0.0",
            "description": "d",
            "scripts": {"postinstall": "node x", "test": "t"},
            "dependencies": {"a": "1"},
            "dist": {
                "tarball": "https://h.example/npm/repo/@scope/pkg/-/pkg-1.0.0.tgz",
                "integrity": "sha512-AAAA",
                "shasum": "ab",
                "fileCount": 3
            }
        }
    });
    assert_eq!(actual, Ok(expected));
}

#[test]
fn render_upstream_never_copies_foreign_url_into_output() {
    // Arrange
    let doc = upstream_doc();

    // Act
    let actual = render(&doc, View::Full).map(|out| out.to_string().contains("evil.example"));

    // Assert
    assert_eq!(actual, Ok(false));
}

#[test]
fn render_upstream_abbreviated_projects_whitelist_and_derives_install_script_flag() {
    // Arrange
    let doc = upstream_doc();

    // Act
    let actual = render(&doc, View::Abbreviated);

    // Assert
    assert_eq!(
        actual,
        Ok(json!({
            "name": "@scope/pkg",
            "modified": "2024-01-02T03:04:05.000Z",
            "dist-tags": {"latest": "1.0.0"},
            "versions": {
                "1.0.0": {
                    "name": "@scope/pkg",
                    "version": "1.0.0",
                    "hasInstallScript": true,
                    "dependencies": {"a": "1"},
                    "dist": {
                        "tarball": "https://h.example/npm/repo/@scope/pkg/-/pkg-1.0.0.tgz",
                        "integrity": "sha512-AAAA",
                        "shasum": "ab",
                        "fileCount": 3
                    }
                }
            }
        }))
    );
}

#[test]
fn render_upstream_derives_install_script_flag_only_from_install_hooks() {
    // Arrange
    let scripts = [
        json!({"preinstall": "x"}),
        json!({"install": "x"}),
        json!({"postinstall": "x"}),
        json!({"test": "x", "prepare": "y"}),
        json!("not an object"),
    ];
    let docs = scripts.map(|scripts| {
        json!({
            "name": "@scope/pkg",
            "versions": {"1.0.0": {"scripts": scripts, "dist": {"tarball": "u"}}}
        })
    });

    // Act
    let actual = docs.each_ref().map(|doc| {
        render(doc, View::Abbreviated)
            .map(|out| out["versions"]["1.0.0"]["hasInstallScript"].clone())
    });

    // Assert
    assert_eq!(
        actual,
        [
            Ok(json!(true)),
            Ok(json!(true)),
            Ok(json!(true)),
            Ok(Value::Null),
            Ok(Value::Null),
        ]
    );
}

#[test]
fn render_upstream_keeps_explicit_install_script_flag() {
    // Arrange
    let doc = json!({
        "name": "@scope/pkg",
        "versions": {"1.0.0": {
            "hasInstallScript": false,
            "scripts": {"install": "x"},
            "dist": {"tarball": "u"}
        }}
    });

    // Act
    let actual = render(&doc, View::Abbreviated)
        .map(|out| out["versions"]["1.0.0"]["hasInstallScript"].clone());

    // Assert
    assert_eq!(actual, Ok(json!(false)));
}

#[test]
fn render_upstream_error_kinds_for_name_problems() {
    // Arrange
    let foreign = serde_json::to_vec(&json!({"name": "other", "versions": {}})).unwrap();
    let nameless = serde_json::to_vec(&json!({"versions": {}})).unwrap();
    let cases: [&[u8]; 2] = [&foreign, &nameless];

    // Act
    let actual = cases.map(|raw| render_upstream(raw, &pkg("@scope/pkg"), View::Full, BASE));

    // Assert
    assert_eq!(
        actual,
        [Err(DocError::NameMismatch), Err(DocError::MissingName)]
    );
}

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

#[test]
fn render_hosted_full_builds_document_from_state() {
    // Arrange
    let versions = [
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
    ];
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
fn render_hosted_derives_latest_when_tag_is_missing() {
    // Arrange
    let meta =
        |v: &str| format!(r#"{{"name":"left-pad","version":"{v}","dist":{{"integrity":"x"}}}}"#);
    let release_and_newer_pre = [
        stored("left-pad", "1.0.0", 1, 10, None, &meta("1.0.0")),
        stored("left-pad", "1.5.0", 2, 20, None, &meta("1.5.0")),
        stored("left-pad", "2.0.0-rc.1", 3, 30, None, &meta("2.0.0-rc.1")),
    ];
    let only_pre = [
        stored("left-pad", "1.0.0-alpha", 1, 10, None, &meta("1.0.0-alpha")),
        stored("left-pad", "1.0.0-beta", 2, 20, None, &meta("1.0.0-beta")),
    ];
    let cases = [&release_and_newer_pre[..], &only_pre[..]];

    // Act
    let actual =
        cases.map(|versions| hosted(versions, &[], View::Full).map(|doc| doc["dist-tags"].clone()));

    // Assert
    assert_eq!(
        actual,
        [
            Ok(json!({"latest": "1.5.0"})),
            Ok(json!({"latest": "1.0.0-beta"})),
        ]
    );
}

#[test]
fn render_hosted_keeps_explicit_latest_even_if_not_highest() {
    // Arrange
    let meta =
        |v: &str| format!(r#"{{"name":"left-pad","version":"{v}","dist":{{"integrity":"x"}}}}"#);
    let versions = [
        stored("left-pad", "1.0.0", 1, 10, None, &meta("1.0.0")),
        stored("left-pad", "2.0.0", 2, 20, None, &meta("2.0.0")),
    ];
    let tags = [("latest".to_owned(), version_id(1))];

    // Act
    let actual = hosted(&versions, &tags, View::Full).map(|doc| doc["dist-tags"].clone());

    // Assert
    assert_eq!(actual, Ok(json!({"latest": "1.0.0"})));
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
