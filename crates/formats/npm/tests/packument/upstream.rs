//! Документ upstream: перенос в собственный вид, белый список полей, ошибки имён.

use crane_format_npm::{DocError, View, render_upstream};
use serde_json::{Value, json};

use crate::common::pkg;
use crate::{BASE, parse};

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

#[test]
fn render_upstream_drops_dist_tags_pointing_at_dropped_versions() {
    // Arrange
    let mut doc = upstream_doc();
    doc["dist-tags"] =
        json!({"latest": "1.0.0", "next": "1.0.2", "junk": "not a version", "ghost": "9.9.9"});

    // Act
    let actual = render(&doc, View::Full).map(|out| out["dist-tags"].clone());

    // Assert
    assert_eq!(actual, Ok(json!({"latest": "1.0.0"})));
}
