use crane_domain::registry::{Ecosystem, NameError};

use crate::ecosystem::MAVEN;

#[test]
fn group_and_artifact_are_kept_as_written() {
    // Arrange
    let raws = [
        "com.acme:core",
        "Org.Apache:Commons-Lang3",
        "io.github.user-name:lib.api",
        "a:b",
        "com.acme:core_2.13",
    ];

    // Act
    let actual = raws.map(|raw| {
        MAVEN
            .parse_name(raw)
            .map(|name| (name.normalized().to_owned(), name.as_written().to_owned()))
    });

    // Assert
    assert_eq!(actual, raws.map(|raw| Ok((raw.to_owned(), raw.to_owned()))));
}

#[test]
fn malformed_coordinates_are_rejected_with_the_reason() {
    // Arrange
    let too_long = format!("g:{}", "a".repeat(254));
    let cases = [
        ("", NameError::Empty),
        ("acme", NameError::MalformedScope),
        (":core", NameError::MalformedScope),
        ("com.acme:", NameError::MalformedScope),
        ("com..acme:core", NameError::MalformedScope),
        (".acme:core", NameError::MalformedScope),
        ("com.acme.:core", NameError::MalformedScope),
        ("com.acme:co:re", NameError::InvalidChar(':')),
        ("com.ac me:core", NameError::InvalidChar(' ')),
        ("com/acme:core", NameError::InvalidChar('/')),
        ("com.acme:co/re", NameError::InvalidChar('/')),
        ("com.ацме:core", NameError::InvalidChar('а')),
        ("com.acme:.", NameError::Reserved(".".to_owned())),
        ("com.acme:..", NameError::Reserved("..".to_owned())),
        (
            "com.acme:core-SNAPSHOT",
            NameError::Reserved("core-SNAPSHOT".to_owned()),
        ),
        (too_long.as_str(), NameError::TooLong { len: 256, max: 255 }),
    ];

    // Act
    let actual = cases.each_ref().map(|(raw, _)| {
        MAVEN
            .parse_name(raw)
            .map(|name| name.normalized().to_owned())
    });

    // Assert
    assert_eq!(actual, cases.map(|(_, error)| Err(error)));
}

#[test]
fn name_of_maximum_length_is_accepted() {
    // Arrange
    let raw = format!("g:{}", "a".repeat(253));

    // Act
    let result = MAVEN.parse_name(&raw);

    // Assert
    assert_eq!(result.map(|name| name.normalized().len()), Ok(255));
}

#[test]
fn artifact_named_snapshot_without_dash_is_accepted() {
    // Arrange
    let raw = "com.acme:SNAPSHOT";

    // Act
    let result = MAVEN.parse_name(raw);

    // Assert
    assert_eq!(
        result.map(|name| name.normalized().to_owned()),
        Ok(raw.to_owned())
    );
}

#[test]
fn prefix_base_is_a_group_id_without_artifact() {
    // Arrange
    let raws = ["Com.Acme", "com.acme:core"];

    // Act
    let actual = raws.map(|raw| MAVEN.prefix_base(raw));

    // Assert
    assert_eq!(
        actual,
        [Ok("Com.Acme".to_owned()), Err(NameError::InvalidChar(':'))]
    );
}
