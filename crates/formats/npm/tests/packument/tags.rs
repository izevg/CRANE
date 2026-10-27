//! Эффективные dist-теги: вывод latest и отбрасывание висячих.

use crane_format_npm::effective_tags;

use crate::common::{stored, version_id};

fn meta(version: &str) -> String {
    format!(r#"{{"name":"left-pad","version":"{version}","dist":{{"integrity":"x"}}}}"#)
}

fn tags_of(
    versions: &[crane_domain::registry::PackageVersion],
    tags: &[(String, crane_domain::registry::VersionId)],
) -> Vec<(String, String)> {
    effective_tags(versions, tags)
        .into_iter()
        .map(|(tag, version)| (tag.to_owned(), version.as_written().to_owned()))
        .collect()
}

#[test]
fn effective_tags_derive_latest_when_tag_is_missing() {
    // Arrange
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
    let actual = cases.map(|versions| tags_of(versions, &[]));

    // Assert
    let latest = |version: &str| vec![("latest".to_owned(), version.to_owned())];
    assert_eq!(actual, [latest("1.5.0"), latest("1.0.0-beta")]);
}

#[test]
fn effective_tags_keep_explicit_latest_even_if_not_highest() {
    // Arrange
    let versions = [
        stored("left-pad", "1.0.0", 1, 10, None, &meta("1.0.0")),
        stored("left-pad", "2.0.0", 2, 20, None, &meta("2.0.0")),
    ];
    let tags = [("latest".to_owned(), version_id(1))];

    // Act
    let actual = tags_of(&versions, &tags);

    // Assert
    assert_eq!(actual, [("latest".to_owned(), "1.0.0".to_owned())]);
}

#[test]
fn effective_tags_drop_tags_pointing_at_absent_versions() {
    // Arrange
    let versions = [stored("left-pad", "1.0.0", 1, 10, None, &meta("1.0.0"))];
    let tags = [
        ("next".to_owned(), version_id(1)),
        ("ghost".to_owned(), version_id(99)),
    ];

    // Act
    let actual = tags_of(&versions, &tags);

    // Assert
    assert_eq!(
        actual,
        [
            ("latest".to_owned(), "1.0.0".to_owned()),
            ("next".to_owned(), "1.0.0".to_owned()),
        ]
    );
}
