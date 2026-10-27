#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::registry::{Format, Version};
use crane_format_npm::{
    View, WriteError, content_type, parse_tag, parse_tag_body, removable, render_error, render_ok,
    render_tags, render_whoami,
};

fn version(raw: &str) -> Version {
    Version::parse(Format::Npm, raw).unwrap()
}

#[test]
fn parse_tag_accepts_url_safe_non_version_names() {
    // Arrange
    let cases = [
        "latest",
        "next",
        "beta-1",
        "v2.x",
        "a_b.c~d",
        &"a".repeat(128),
    ];

    // Act
    let actual = cases.map(parse_tag);

    // Assert
    assert_eq!(actual, cases.map(|tag| Ok(tag.to_owned())));
}

#[test]
fn parse_tag_refuses_bad_names() {
    // Arrange
    let long = "a".repeat(129);
    let cases = [
        "",
        &long,
        "a b",
        "a/b",
        "é",
        "1.0.0",
        "1.0.0-rc.1",
        "0.0.1+b",
    ];

    // Act
    let actual = cases.map(parse_tag);

    // Assert
    assert_eq!(
        actual,
        [
            Err(WriteError::TagEmpty),
            Err(WriteError::TagTooLong),
            Err(WriteError::TagChar(' ')),
            Err(WriteError::TagChar('/')),
            Err(WriteError::TagChar('é')),
            Err(WriteError::TagIsVersion),
            Err(WriteError::TagIsVersion),
            Err(WriteError::TagChar('+')),
        ]
    );
}

#[test]
fn removable_refuses_only_latest() {
    // Arrange
    let cases = ["latest", "next", "Latest"];

    // Act
    let actual = cases.map(removable);

    // Assert
    assert_eq!(actual, [Err(WriteError::LatestRemoval), Ok(()), Ok(())]);
}

#[test]
fn parse_tag_body_reads_json_string_version() {
    // Arrange
    let cases: [&[u8]; 4] = [br#""1.2.3""#, br#""1.2""#, b"1.2.3", br#"{"v":"1.2.3"}"#];

    // Act
    let actual = cases.map(|body| {
        parse_tag_body(body)
            .map(|v| v.as_written().to_owned())
            .map_err(|_| ())
    });

    // Assert
    assert_eq!(actual, [Ok("1.2.3".to_owned()), Err(()), Err(()), Err(())]);
}

#[test]
fn render_tags_is_sorted_json_object() {
    // Arrange
    let (one, two) = (version("1.0.0"), version("2.0.0-rc.1"));
    let tags = [("next", &two), ("latest", &one)];

    // Act
    let actual = render_tags(tags);

    // Assert
    assert_eq!(actual, r#"{"latest":"1.0.0","next":"2.0.0-rc.1"}"#);
}

#[test]
fn view_is_abbreviated_only_when_client_asks_for_it() {
    // Arrange
    let cases = [
        None,
        Some("application/json"),
        Some("*/*"),
        Some("application/vnd.npm.install-v1+json"),
        Some("application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*"),
    ];

    // Act
    let actual = cases.map(View::from_accept);

    // Assert
    assert_eq!(
        actual,
        [
            View::Full,
            View::Full,
            View::Full,
            View::Abbreviated,
            View::Abbreviated
        ]
    );
}

#[test]
fn content_type_follows_view() {
    // Arrange
    let views = [View::Full, View::Abbreviated];

    // Act
    let actual = views.map(content_type);

    // Assert
    assert_eq!(
        actual,
        ["application/json", "application/vnd.npm.install-v1+json"]
    );
}

#[test]
fn replies_are_npm_json_bodies() {
    // Arrange
    let detail = "плохой \"запрос\"";

    // Act
    let actual = [render_error(detail), render_ok(), render_whoami("alice")];

    // Assert
    assert_eq!(
        actual,
        [
            r#"{"error":"плохой \"запрос\""}"#.to_owned(),
            r#"{"ok":true}"#.to_owned(),
            r#"{"username":"alice"}"#.to_owned(),
        ]
    );
}
