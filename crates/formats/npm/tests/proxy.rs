#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use common::pkg;
use crane_domain::proxy::{Freshness, UpstreamPath};
use crane_domain::registry::{Checksum, Sha512};
use crane_format_npm::{Tarball, TarballError, packument_path, tarball};
use serde_json::{Value, json};

fn sri(byte: u8) -> String {
    format!("sha512-{}", STANDARD.encode([byte; 64]))
}

fn doc(name: &str, dist: &Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "name": name,
        "versions": {"1.0.0": {"name": name, "version": "1.0.0", "dist": dist}}
    }))
    .unwrap()
}

#[allow(clippy::unnecessary_wraps, reason = "сравнивается с Result из tarball")]
fn found(path: &str, byte: u8) -> Result<Tarball, TarballError> {
    Ok(Tarball {
        path: UpstreamPath::parse(path).unwrap(),
        expected: Checksum::Sha512(Sha512::from([byte; 64])),
    })
}

#[test]
fn tarball_expects_sha512_from_integrity_and_builds_path_from_request() {
    // Arrange
    let plain = doc(
        "left-pad",
        &json!({"integrity": sri(1), "shasum": "ab".repeat(20), "tarball": "https://evil.example/x.tgz"}),
    );
    let scoped = doc(
        "@scope/pkg",
        &json!({"integrity": format!("sha1-{} {}", STANDARD.encode([3u8; 20]), sri(2))}),
    );
    let cases = [
        (plain, pkg("left-pad"), "left-pad-1.0.0.tgz"),
        (scoped, pkg("@scope/pkg"), "pkg-1.0.0.tgz"),
    ];

    // Act
    let actual = cases.map(|(raw, package, file)| tarball(&raw, &package, file));

    // Assert
    assert_eq!(
        actual,
        [
            found("left-pad/-/left-pad-1.0.0.tgz", 1),
            found("@scope/pkg/-/pkg-1.0.0.tgz", 2),
        ]
    );
}

#[test]
fn tarball_refuses_versions_without_sha512() {
    // Arrange
    let sha1 = format!("sha1-{}", STANDARD.encode([3u8; 20]));
    let cases = [
        doc("left-pad", &json!({"shasum": "ab".repeat(20)})),
        doc(
            "left-pad",
            &json!({"integrity": sha1, "shasum": "ab".repeat(20)}),
        ),
        doc(
            "left-pad",
            &json!({"integrity": "sha384-AAAA", "shasum": "ab".repeat(20)}),
        ),
        doc(
            "left-pad",
            &json!({"integrity": "sha512-%%%", "shasum": "ab".repeat(20)}),
        ),
        doc("left-pad", &json!({"tarball": "https://x.example/a.tgz"})),
        doc("left-pad", &json!({})),
    ];

    // Act
    let actual = cases
        .each_ref()
        .map(|raw| tarball(raw, &pkg("left-pad"), "left-pad-1.0.0.tgz"));

    // Assert
    assert_eq!(actual, cases.map(|_| Err(TarballError::WeakIntegrity)));
}

#[test]
fn tarball_refuses_unlisted_files_and_foreign_documents() {
    // Arrange
    let good = doc("left-pad", &json!({"integrity": sri(1)}));
    let foreign = doc("other", &json!({"integrity": sri(1)}));
    let cases = [
        (good.clone(), "left-pad-9.9.9.tgz"),
        (good.clone(), "left-pad-1.0.0.tar"),
        (good.clone(), "other-1.0.0.tgz"),
        (good.clone(), "../left-pad-1.0.0.tgz"),
        (foreign, "left-pad-1.0.0.tgz"),
        (b"not json".to_vec(), "left-pad-1.0.0.tgz"),
    ];

    // Act
    let actual = cases
        .each_ref()
        .map(|(raw, file)| tarball(raw, &pkg("left-pad"), file));

    // Assert
    assert_eq!(actual, cases.map(|_| Err(TarballError::NotListed)));
}

#[test]
fn packument_path_is_one_segment_with_escaped_scope_slash() {
    // Arrange
    let packages = [pkg("left-pad"), pkg("@scope/pkg"), pkg("JSONStream")];

    // Act
    let actual = packages
        .each_ref()
        .map(|package| packument_path(package).map(|p| p.as_str().to_owned()));

    // Assert
    assert_eq!(
        actual,
        [
            Ok("left-pad".to_owned()),
            Ok("@scope%2fpkg".to_owned()),
            Ok("JSONStream".to_owned())
        ]
    );
}

#[test]
fn packument_serves_stale_for_five_minutes_after_ttl() {
    // Arrange
    let fetched_at = std::time::SystemTime::UNIX_EPOCH;
    let at = |secs| fetched_at + std::time::Duration::from_secs(secs);
    let ttl = 5 * 60;
    let ages = [ttl, ttl + 5 * 60 - 1, ttl + 5 * 60];

    // Act
    let actual = ages.map(|age| crane_format_npm::PACKUMENT.assess(0, None, fetched_at, at(age)));

    // Assert
    assert_eq!(
        actual,
        [Freshness::Stale, Freshness::Stale, Freshness::Expired]
    );
}
