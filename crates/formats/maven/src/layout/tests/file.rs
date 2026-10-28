use crate::layout::{FileName, RouteError};

fn name<'a>(version: &'a str, classifier: Option<&'a str>, extension: &'a str) -> FileName<'a> {
    FileName {
        version,
        classifier,
        extension,
    }
}

#[test]
fn release_file_names_yield_version_classifier_and_extension() {
    // Arrange
    let cases = [
        ("core", "1.0", "core-1.0.jar", name("1.0", None, "jar")),
        ("core", "1.0", "core-1.0.pom", name("1.0", None, "pom")),
        (
            "core",
            "1.0",
            "core-1.0.module",
            name("1.0", None, "module"),
        ),
        (
            "core",
            "1.0",
            "core-1.0-sources.jar",
            name("1.0", Some("sources"), "jar"),
        ),
        (
            "core",
            "1.0",
            "core-1.0.tar.gz",
            name("1.0", None, "tar.gz"),
        ),
        (
            "core",
            "1.0",
            "core-1.0.jar.asc",
            name("1.0", None, "jar.asc"),
        ),
        (
            "core",
            "1.0",
            "core-1.0-bin.tar.gz",
            name("1.0", Some("bin"), "tar.gz"),
        ),
        (
            "core",
            "1.0",
            "core-1.0.sigstore.json",
            name("1.0", None, "sigstore.json"),
        ),
        (
            "spring-core",
            "5.3.1-rc1",
            "spring-core-5.3.1-rc1-tests.jar",
            name("5.3.1-rc1", Some("tests"), "jar"),
        ),
    ];

    // Act
    let actual = cases.map(|(artifact, dir, file, _)| FileName::parse(artifact, dir, file));

    // Assert
    assert_eq!(actual, cases.map(|(.., expected)| Ok(expected)));
}

#[test]
fn snapshot_file_names_carry_the_timestamp_build_of_their_directory() {
    // Arrange
    let cases = [
        (
            "core",
            "1.0-SNAPSHOT",
            "core-1.0-20261028.123456-3.jar",
            name("1.0-20261028.123456-3", None, "jar"),
        ),
        (
            "core",
            "1.0-SNAPSHOT",
            "core-1.0-20261028.123456-3-sources.jar",
            name("1.0-20261028.123456-3", Some("sources"), "jar"),
        ),
        (
            "core",
            "1.0-SNAPSHOT",
            "core-1.0-20261028.123456-13.pom.asc",
            name("1.0-20261028.123456-13", None, "pom.asc"),
        ),
        (
            "spring-core",
            "5.3-SNAPSHOT",
            "spring-core-5.3-20261028.123456-3-tests.jar",
            name("5.3-20261028.123456-3", Some("tests"), "jar"),
        ),
        (
            "core",
            "1.0-beta-SNAPSHOT",
            "core-1.0-beta-20261028.123456-3.jar",
            name("1.0-beta-20261028.123456-3", None, "jar"),
        ),
    ];

    // Act
    let actual = cases.map(|(artifact, dir, file, _)| FileName::parse(artifact, dir, file));

    // Assert
    assert_eq!(actual, cases.map(|(.., expected)| Ok(expected)));
}

#[test]
fn file_names_that_do_not_match_artifact_or_directory_are_rejected() {
    // Arrange
    let cases = [
        ("core", "1.0", "other-1.0.jar"),
        ("core", "1.0", "core-1.1.jar"),
        ("core", "1.0", "core-1.0"),
        ("core", "1.0", "core-1.0-.jar"),
        ("core", "1.0", "core-1.0.jar."),
        ("core", "1.0", "core-1.0..jar"),
        ("core", "1.0", "core-1.0.ja r"),
        ("core", "1.0", "core-1.0-bad class.jar"),
        ("core", "1.0", "core1.0.jar"),
        ("core", "1", "core-10.jar"),
        ("core", "1.0-SNAPSHOT", "core-1.0.jar"),
        ("core", "1.0-SNAPSHOT", "core-1.1-20261028.123456-3.jar"),
        ("core", "1.0-SNAPSHOT", "core-1.0-2026102.123456-3.jar"),
        ("core", "1.0-SNAPSHOT", "core-1.0-20261028.123456-.jar"),
        ("core", "1.0-SNAPSHOT", "core-1.0-20261028.123456-3"),
        ("core", "1.0-SNAPSHOT", "core-1.0-2026102é.123456-3.jar"),
        ("core", "-SNAPSHOT", "core--20261028.123456-3.jar"),
    ];

    // Act
    let actual = cases.map(|(artifact, dir, file)| FileName::parse(artifact, dir, file));

    // Assert
    assert_eq!(actual, cases.map(|_| Err(RouteError::FileMismatch)));
}

#[test]
fn non_unique_snapshot_names_are_rejected_with_their_own_error() {
    // Arrange
    let files = ["core-1.0-SNAPSHOT.jar", "core-1.0-SNAPSHOT-sources.jar"];

    // Act
    let actual = files.map(|file| FileName::parse("core", "1.0-SNAPSHOT", file));

    // Assert
    assert_eq!(actual, files.map(|_| Err(RouteError::NotUniqueSnapshot)));
}

#[test]
fn timestamp_build_is_rejected_as_a_release_directory() {
    // Arrange
    let dir = "1.0-20261028.123456-3";

    // Act
    let result = FileName::parse("core", dir, "core-1.0-20261028.123456-3.jar");

    // Assert
    assert_eq!(result, Err(RouteError::BuildOutsideSnapshot));
}
