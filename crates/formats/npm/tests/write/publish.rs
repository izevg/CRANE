//! Классификация запроса публикации: тело, суммы, теги, зависимости.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bytes::Bytes;
use crane_domain::registry::{Checksum, FormatMetadata, Sha1, Sha512};
use crane_format_npm::{NpmPublish, WriteError, classify};
use serde_json::json;

use crate::common::{pkg, version};
use crate::{TARBALL, integrity, manifest, publish_for, publish_result, with, with_manifest};

#[test]
fn classify_publish_extracts_tarball_declared_sums_tag_and_clean_metadata() {
    // Arrange
    let request = with(|_| {});

    // Act
    let actual = publish_result(&request);

    // Assert
    let expected_metadata = format!(
        r#"{{"_hasShrinkwrap":false,"dependencies":{{"a":"^1.0.0"}},"dist":{{"integrity":{},"shasum":"{}"}},"name":"left-pad","version":"1.0.0"}}"#,
        serde_json::to_string(&integrity()).unwrap(),
        "ab".repeat(20),
    );
    assert_eq!(
        actual,
        Ok(NpmPublish {
            version: version("1.0.0"),
            file: "left-pad-1.0.0.tgz".to_owned(),
            tarball: Bytes::from_static(TARBALL),
            declared: vec![
                Checksum::Sha512(Sha512::from([1u8; 64])),
                Checksum::Sha1(Sha1::from([0xab; 20])),
            ],
            tag: "latest".to_owned(),
            metadata: FormatMetadata::new(expected_metadata),
        })
    );
}

#[test]
fn classify_publish_accepts_scoped_package_and_sri_without_padding_and_sha1_entry() {
    // Arrange
    let sri = format!(
        "sha512-{} sha1-{}",
        STANDARD.encode([1u8; 64]).trim_end_matches('='),
        STANDARD.encode([3u8; 20])
    );
    let request = serde_json::to_vec(&json!({
        "name": "@scope/pkg",
        "dist-tags": {"next": "2.0.0-rc.1"},
        "versions": {"2.0.0-rc.1": {
            "name": "@scope/pkg", "version": "2.0.0-rc.1", "dist": {"integrity": sri}
        }},
        "_attachments": {"pkg-2.0.0-rc.1.tgz": {"data": STANDARD.encode(TARBALL)}},
    }))
    .unwrap();

    // Act
    let actual = publish_for("@scope/pkg", &request);

    // Assert
    assert_eq!(
        actual.map(|publish| (publish.declared, publish.tag, publish.file)),
        Ok((
            vec![
                Checksum::Sha512(Sha512::from([1u8; 64])),
                Checksum::Sha1(Sha1::from([3u8; 20])),
            ],
            "next".to_owned(),
            "pkg-2.0.0-rc.1.tgz".to_owned(),
        ))
    );
}

#[test]
fn classify_publish_refuses_inconsistent_requests() {
    // Arrange
    let cases = [
        with(|b| b["name"] = json!("other")),
        with(|b| b["versions"]["1.0.0"]["name"] = json!("other")),
        with(|b| b["versions"]["1.0.0"]["version"] = json!("1.0.1")),
        with(|b| b["_attachments"]["left-pad-1.0.0.tgz"]["length"] = json!(1)),
        with(|b| {
            let attachment = b["_attachments"]["left-pad-1.0.0.tgz"].take();
            b["_attachments"] = json!({"left-pad-9.9.9.tgz": attachment});
        }),
        with(|b| b["_attachments"]["left-pad-1.0.0.tgz"]["data"] = json!("не base64!")),
        with(|b| b["versions"]["1.0.1"] = manifest()),
        with(|b| b["_attachments"]["left-pad-1.0.1.tgz"] = json!({"data": ""})),
        with(|b| b["dist-tags"] = json!({"latest": "2.0.0"})),
        with(|b| b["dist-tags"] = json!({"latest": "1.0.0", "next": "1.0.0"})),
        with(|b| b["dist-tags"] = json!({})),
        with(|b| b["dist-tags"] = json!({"1.0.0": "1.0.0"})),
    ];

    // Act
    let actual = cases.map(|request| publish_result(&request).map(|_| ()));

    // Assert
    assert_eq!(
        actual,
        [
            Err(WriteError::NameMismatch),
            Err(WriteError::NameMismatch),
            Err(WriteError::VersionMismatch),
            Err(WriteError::LengthMismatch {
                declared: 1,
                actual: 13
            }),
            Err(WriteError::AttachmentMismatch),
            Err(WriteError::Base64),
            Err(WriteError::NotSingleVersion),
            Err(WriteError::NotSingleAttachment),
            Err(WriteError::BadDistTags),
            Err(WriteError::BadDistTags),
            Err(WriteError::BadDistTags),
            Err(WriteError::TagIsVersion),
        ]
    );
}

#[test]
fn classify_publish_without_versions_or_attachments_part_is_refused() {
    // Arrange
    let cases = [
        with(|b| {
            b.as_object_mut().unwrap().remove("versions");
        }),
        with(|b| b["versions"] = json!({})),
        with(|b| b["_attachments"] = json!({})),
    ];

    // Act
    let actual = cases.map(|request| publish_result(&request).map(|_| ()));

    // Assert
    assert_eq!(
        actual,
        [
            Err(WriteError::NotSingleVersion),
            Err(WriteError::NotSingleVersion),
            Err(WriteError::NotSingleAttachment),
        ]
    );
}

#[test]
fn classify_publish_requires_valid_sha512_integrity_and_sha1_shasum() {
    // Arrange
    let sha1_only = format!("sha1-{}", STANDARD.encode([3u8; 20]));
    let short_sha512 = format!("sha512-{}", STANDARD.encode([1u8; 63]));
    let cases = [
        with_manifest(|m| {
            m["dist"].as_object_mut().unwrap().remove("integrity");
        }),
        with_manifest(|m| {
            m.as_object_mut().unwrap().remove("dist");
        }),
        with_manifest(|m| m["dist"]["integrity"] = json!(sha1_only)),
        with_manifest(|m| m["dist"]["integrity"] = json!("sha384-AAAA")),
        with_manifest(|m| m["dist"]["integrity"] = json!(short_sha512)),
        with_manifest(|m| m["dist"]["integrity"] = json!("sha512-%%%")),
        with_manifest(|m| m["dist"]["integrity"] = json!("garbage")),
        with_manifest(|m| m["dist"]["shasum"] = json!("zz")),
    ];

    // Act
    let actual = cases.map(|request| publish_result(&request).map(|_| ()));

    // Assert
    assert_eq!(
        actual,
        [
            Err(WriteError::MissingIntegrity),
            Err(WriteError::MissingIntegrity),
            Err(WriteError::MissingSha512),
            Err(WriteError::MissingSha512),
            Err(WriteError::BadIntegrity),
            Err(WriteError::BadIntegrity),
            Err(WriteError::BadIntegrity),
            Err(WriteError::BadShasum),
        ]
    );
}

#[test]
fn classify_publish_refuses_dependencies_that_leave_the_registry() {
    // Arrange
    let specs = [
        "http://x.example/a.tgz",
        "https://x.example/a.tgz",
        "git+ssh://git@github.com/a/b.git",
        "git://github.com/a/b",
        "github:a/b",
        "gitlab:a/b",
        "a/b",
        "file:../x",
        "link:../x",
        "workspace:*",
        "npm:foo@git+https://x.example/a",
        "npm:@s/n@https://x.example/a",
        "npm:a/b@1.0.0",
        "npm:",
        "foo.tgz",
        "foo.TGZ",
        "foo.tar",
        "foo.tar.gz",
        "..\\x",
        "C:x",
        "npm:left-pad@1.0.0.tgz",
        "npm:left-pad@x.TAR.GZ",
    ];
    let cases = specs.map(|spec| with_manifest(|m| m["dependencies"] = json!({"dep": spec})));

    // Act
    let actual = cases.map(|request| publish_result(&request).map(|_| ()));

    // Assert
    assert_eq!(
        actual,
        specs.map(|spec| Err(WriteError::ForeignDependency {
            name: "dep".to_owned(),
            spec: spec.to_owned(),
        }))
    );
}

#[test]
fn classify_publish_accepts_ranges_tags_and_registry_aliases() {
    // Arrange
    let specs = [
        "^1.2.3",
        ">=1 <2 || 3.x",
        "1.0.0 - 2.0.0",
        "latest",
        "*",
        "",
        "npm:left-pad@^1.0.0",
        "npm:@scope/name@latest",
        "npm:left-pad",
        "npm:@scope/name",
    ];
    let cases = specs.map(|spec| with_manifest(|m| m["dependencies"] = json!({"dep": spec})));

    // Act
    let actual = cases.map(|request| publish_result(&request).map(|_| ()));

    // Assert
    assert_eq!(actual, specs.map(|_| Ok(())));
}

#[test]
fn classify_publish_checks_every_installed_dependency_section_but_not_dev() {
    // Arrange
    let cases = [
        "dependencies",
        "optionalDependencies",
        "peerDependencies",
        "devDependencies",
    ]
    .map(|section| with_manifest(|m| m[section] = json!({"dep": "github:a/b"})));

    // Act
    let actual = cases.map(|request| publish_result(&request).map(|_| ()));

    // Assert
    let foreign = Err(WriteError::ForeignDependency {
        name: "dep".to_owned(),
        spec: "github:a/b".to_owned(),
    });
    assert_eq!(actual, [foreign.clone(), foreign.clone(), foreign, Ok(())]);
}

#[test]
fn classify_reports_broken_json() {
    // Arrange
    let request = b"{not json";

    // Act
    let actual = classify(&pkg("left-pad"), request, &[]);

    // Assert
    assert!(matches!(actual, Err(WriteError::Json(_))));
}
