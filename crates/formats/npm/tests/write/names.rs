//! Политика имён новых пакетов.

use crane_format_npm::{WriteError, classify, valid_for_new};

use crate::common::pkg;
use crate::with;

#[test]
fn classify_publish_refuses_legacy_names_for_new_packages() {
    // Arrange
    let names = [
        "JSONStream",
        "Left-Pad",
        "a~b",
        "a'b",
        "a!b",
        "a(b)",
        "a*b",
        "@Scope/n",
    ];

    // Act
    let actual = names.map(|raw| valid_for_new(&pkg(raw)));

    // Assert
    assert_eq!(
        actual,
        [
            Err(WriteError::NewNameChar('J')),
            Err(WriteError::NewNameChar('L')),
            Err(WriteError::NewNameChar('~')),
            Err(WriteError::NewNameChar('\'')),
            Err(WriteError::NewNameChar('!')),
            Err(WriteError::NewNameChar('(')),
            Err(WriteError::NewNameChar('*')),
            Err(WriteError::NewNameChar('S')),
        ]
    );
}

#[test]
fn valid_for_new_accepts_lowercase_names() {
    // Arrange
    let names = ["left-pad", "@scope/name", "a.b_c-d9"];

    // Act
    let actual = names.map(|raw| valid_for_new(&pkg(raw)));

    // Assert
    assert_eq!(actual, [Ok(()), Ok(()), Ok(())]);
}

#[test]
fn classify_publish_applies_new_name_policy() {
    // Arrange
    let request = with(|_| {});

    // Act
    let actual = classify(&pkg("Left-Pad"), &request, &[]).map(|_| ());

    // Assert
    assert_eq!(actual, Err(WriteError::NewNameChar('L')));
}
