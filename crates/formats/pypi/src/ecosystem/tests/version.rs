use crane_domain::registry::{Version, VersionError};

use crate::ecosystem::parse_version;

fn invalid(raw: &str) -> VersionError {
    parse_version(raw).unwrap_err()
}

#[test]
fn canonical_form_drops_trailing_zeros_and_normalizes_the_rest() {
    // Arrange
    let raws = [
        "1.0.0",
        "1.0",
        "10.0.0",
        "0.0.0",
        "1.0a1",
        "1.0.0-alpha.1",
        "1!2.0.post1",
        "1.0.0.dev0",
        "1.0-1",
        "1.0+Local_1",
        "v1.2.0RC3",
    ];

    // Act
    let actual = raws.map(|raw| parse_version(raw).unwrap().canonical().to_owned());

    // Assert
    let expected = [
        "1",
        "1",
        "10",
        "0",
        "1a1",
        "1a1",
        "1!2.post1",
        "1.dev0",
        "1.post1",
        "1+local.1",
        "1.2rc3",
    ]
    .map(str::to_owned);
    assert_eq!(actual, expected);
}

#[test]
fn written_form_is_what_the_client_sent() {
    // Arrange
    let raw = "1.0.0-Alpha1";

    // Act
    let version = parse_version(raw).unwrap();

    // Assert
    assert_eq!(version.as_written(), raw);
}

#[test]
fn versions_equal_under_pep_440_are_one_version_with_one_order_key() {
    // Arrange
    let (wheel, sdist) = (
        parse_version("1.0").unwrap(),
        parse_version("1.0.0").unwrap(),
    );

    // Act
    let same = (
        wheel.canonical() == sdist.canonical(),
        wheel.order_key() == sdist.order_key(),
    );

    // Assert
    assert_eq!(same, (true, true));
}

#[test]
fn dev_releases_are_prereleases_and_post_releases_are_not() {
    // Arrange
    let raws = [
        "1.0",
        "1.0a1",
        "1.0rc1",
        "1.0.dev1",
        "1.0.post1",
        "1.0.post1.dev1",
    ];

    // Act
    let actual = raws.map(|raw| parse_version(raw).unwrap().is_prerelease());

    // Assert
    assert_eq!(actual, [false, true, true, true, false, true]);
}

#[test]
fn malformed_versions_are_rejected_as_invalid() {
    // Arrange
    let raws = ["", "latest", "1..0", "1.0+", "1.0 beta x"];

    // Act
    let actual = raws.map(|raw| matches!(invalid(raw), VersionError::Invalid { .. }));

    // Assert
    assert_eq!(actual, [true; 5]);
}

#[test]
fn release_numbers_beyond_u64_are_rejected() {
    // Arrange
    let raw = "99999999999999999999999.0";

    // Act
    let result: Result<Version, VersionError> = parse_version(raw);

    // Assert
    assert!(matches!(result, Err(VersionError::Invalid { .. })));
}
