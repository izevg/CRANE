//! Ключи вложений пакета с областью.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crane_format_npm::WriteError;
use serde_json::json;

use crate::{TARBALL, integrity, publish_for};

fn scoped_body(attachment_key: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "name": "@scope/pkg",
        "dist-tags": {"latest": "1.0.0"},
        "versions": {"1.0.0": {
            "name": "@scope/pkg", "version": "1.0.0", "dist": {"integrity": integrity()}
        }},
        "_attachments": {attachment_key: {"data": STANDARD.encode(TARBALL)}},
    }))
    .unwrap()
}

#[test]
fn classify_publish_accepts_full_and_basename_attachment_keys_for_scoped_package() {
    // Arrange
    let keys = ["@scope/pkg-1.0.0.tgz", "pkg-1.0.0.tgz"];
    let requests = keys.map(scoped_body);

    // Act
    let actual = requests
        .each_ref()
        .map(|request| publish_for("@scope/pkg", request).map(|p| p.file));

    // Assert
    assert_eq!(actual, keys.map(|_| Ok("pkg-1.0.0.tgz".to_owned())));
}

#[test]
fn classify_publish_refuses_other_attachment_keys_for_scoped_package() {
    // Arrange
    let keys = [
        "@scope/other-1.0.0.tgz",
        "@other/pkg-1.0.0.tgz",
        "@scope/pkg-1.0.1.tgz",
        "scope/pkg-1.0.0.tgz",
        "pkg-1.0.0.tar",
    ];
    let requests = keys.map(scoped_body);

    // Act
    let actual = requests
        .each_ref()
        .map(|request| publish_for("@scope/pkg", request).map(|_| ()));

    // Assert
    assert_eq!(actual, keys.map(|_| Err(WriteError::AttachmentMismatch)));
}
