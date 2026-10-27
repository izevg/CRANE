#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::registry::{Format, PackageName};
use crane_format_cargo::{index_path, parse_index_path};

fn name(raw: &str) -> PackageName {
    PackageName::parse(Format::Cargo, raw).unwrap()
}

#[test]
fn index_path_follows_cargo_layout_for_each_length() {
    // Arrange
    let raws = ["a", "ab", "abc", "abcd", "abcde"];

    // Act
    let paths: Vec<String> = raws.iter().map(|r| index_path(&name(r))).collect();

    // Assert
    assert_eq!(
        paths,
        ["1/a", "2/ab", "3/a/abc", "ab/cd/abcd", "ab/cd/abcde"]
    );
}

#[test]
fn index_path_lowercases_and_keeps_separators_as_written() {
    // Arrange
    let raws = ["Serde", "foo-bar", "foo_bar", "Ab-C"];

    // Act
    let paths: Vec<String> = raws.iter().map(|r| index_path(&name(r))).collect();

    // Assert
    assert_eq!(
        paths,
        [
            "se/rd/serde",
            "fo/o-/foo-bar",
            "fo/o_/foo_bar",
            "ab/-c/ab-c"
        ]
    );
}

#[test]
fn parse_index_path_accepts_canonical_paths() {
    // Arrange
    let paths = ["1/a", "2/ab", "3/a/abc", "ab/cd/abcd", "se/rd/serde"];

    // Act
    let parsed: Vec<Option<String>> = paths
        .iter()
        .map(|p| parse_index_path(p).map(|n| n.as_written().to_owned()))
        .collect();

    // Assert
    let expected: Vec<Option<String>> = ["a", "ab", "abc", "abcd", "serde"]
        .iter()
        .map(|s| Some((*s).to_owned()))
        .collect();
    assert_eq!(parsed, expected);
}

#[test]
fn parse_index_path_rejects_non_canonical_paths() {
    // Arrange
    let paths = [
        "",
        "/",
        "serde",
        "xx/rd/serde",
        "se/xx/serde",
        "3/b/abc",
        "2/abc",
        "se/rd/Serde",
        "3/a/ABC",
        "extra/se/rd/serde",
        "se/rd/serde/",
        "se/serde",
        "1/1a",
        "1/-",
    ];

    // Act
    let accepted: Vec<&str> = paths
        .into_iter()
        .filter(|p| parse_index_path(p).is_some())
        .collect();

    // Assert
    assert!(accepted.is_empty(), "приняты: {accepted:?}");
}

#[test]
fn dash_and_underscore_paths_parse_to_equal_names() {
    // Arrange
    let (dash, underscore) = ("fo/o-/foo-bar", "fo/o_/foo_bar");

    // Act
    let (a, b) = (parse_index_path(dash), parse_index_path(underscore));

    // Assert
    assert_eq!(a, b);
    assert!(a.is_some());
}
