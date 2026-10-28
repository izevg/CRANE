use crane_domain::registry::{NameError, PackageName};

use crate::ecosystem::parse_name;

fn npm(raw: &str) -> Result<PackageName, NameError> {
    parse_name(raw)
}

#[test]
fn legacy_and_scoped_npm_names_are_accepted_as_written() {
    // Arrange
    let raws = [
        "left-pad",
        "JSONStream",
        "@babel/core",
        "@Scope/Name",
        "a.b_c~d!e'f(g)h*i",
        "@types/node",
    ];

    // Act
    let actual =
        raws.map(|r| npm(r).map(|n| (n.as_written().to_owned(), n.normalized().to_owned())));

    // Assert
    assert_eq!(actual, raws.map(|r| Ok((r.to_owned(), r.to_owned()))));
}

#[test]
fn npm_names_differing_in_case_are_different() {
    // Arrange
    let (a, b) = (npm("JSONStream").unwrap(), npm("jsonstream").unwrap());

    // Act
    let equal = a == b;

    // Assert
    assert!(!equal);
}

#[test]
fn malformed_npm_names_are_rejected_with_reason() {
    // Arrange
    let raws = [
        "",
        "@",
        "@scope",
        "@/name",
        "@scope/",
        "@a/b/c",
        "a/b",
        ".x",
        "_x",
        "@.s/n",
        "@s/_n",
        " a",
        "a ",
        "a b",
        "a@b",
        "é",
        "node_modules",
        "favicon.ico",
    ];

    // Act
    let actual = raws.map(|r| npm(r).unwrap_err());

    // Assert
    assert_eq!(
        actual,
        [
            NameError::Empty,
            NameError::MalformedScope,
            NameError::MalformedScope,
            NameError::MalformedScope,
            NameError::MalformedScope,
            NameError::InvalidChar('/'),
            NameError::InvalidChar('/'),
            NameError::InvalidStart('.'),
            NameError::InvalidStart('_'),
            NameError::InvalidStart('.'),
            NameError::InvalidStart('_'),
            NameError::InvalidChar(' '),
            NameError::InvalidChar(' '),
            NameError::InvalidChar(' '),
            NameError::InvalidChar('@'),
            NameError::InvalidChar('é'),
            NameError::Reserved("node_modules".to_owned()),
            NameError::Reserved("favicon.ico".to_owned()),
        ]
    );
}

#[test]
fn npm_name_length_limit_is_214_bytes() {
    // Arrange
    let cases = ["a".repeat(214), "a".repeat(215)];

    // Act
    let actual = cases.each_ref().map(|raw| npm(raw).map(|_| ()));

    // Assert
    assert_eq!(
        actual,
        [Ok(()), Err(NameError::TooLong { len: 215, max: 214 })]
    );
}
