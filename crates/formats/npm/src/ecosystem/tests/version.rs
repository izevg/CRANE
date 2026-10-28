use std::cmp::Ordering;

use crane_domain::registry::Version;

use crate::ecosystem::parse_version;

fn npm(raw: &str) -> Version {
    parse_version(raw).unwrap()
}

#[test]
fn npm_versions_follow_the_same_semver_rules() {
    // Arrange
    let raws = ["1.0.0+a", "1.0.0+b", "1.0", "v1.0.0"];

    // Act
    let actual = raws.map(|r| {
        parse_version(r)
            .map(|v| v.canonical().to_owned())
            .map_err(|_| ())
    });

    // Assert
    assert_eq!(
        actual,
        [
            Ok("1.0.0".to_owned()),
            Ok("1.0.0".to_owned()),
            Err(()),
            Err(())
        ]
    );
}

#[test]
fn prerelease_is_detected_only_by_pre_part() {
    // Arrange
    let versions = ["1.0.0", "1.0.0+build-1", "1.0.0-rc.1", "1.0.0-rc.1+b"].map(npm);

    // Act
    let actual = versions.each_ref().map(Version::is_prerelease);

    // Assert
    assert_eq!(actual, [false, false, true, true]);
}

/// Порядок из semver.org и граничные случаи идентификаторов, по возрастанию.
const SEMVER_CHAIN: [&str; 21] = [
    "0.0.1",
    "0.1.0",
    "1.0.0-0",
    "1.0.0-1",
    "1.0.0-2",
    "1.0.0-10",
    "1.0.0-99999999999999999999",
    "1.0.0-100000000000000000000",
    "1.0.0-alpha",
    "1.0.0-alpha.1",
    "1.0.0-alpha.100",
    "1.0.0-alpha.a1",
    "1.0.0-alpha.beta",
    "1.0.0-beta",
    "1.0.0-beta.2",
    "1.0.0-beta.11",
    "1.0.0-rc.1",
    "1.0.0",
    "1.9.0",
    "1.10.0",
    "256.0.0",
];

#[test]
fn order_key_matches_semver_crate_ordering_on_every_pair() {
    // Arrange
    let pairs: Vec<(&str, &str)> = SEMVER_CHAIN
        .iter()
        .flat_map(|a| SEMVER_CHAIN.iter().map(move |b| (*a, *b)))
        .collect();

    // Act
    let actual: Vec<Ordering> = pairs
        .iter()
        .map(|(a, b)| npm(a).precedence(&npm(b)))
        .collect();

    // Assert
    let expected: Vec<Ordering> = pairs
        .iter()
        .map(|(a, b)| {
            semver::Version::parse(a)
                .unwrap()
                .cmp(&semver::Version::parse(b).unwrap())
        })
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn release_is_greater_than_every_prerelease_of_the_same_core() {
    // Arrange
    let release = npm("1.0.0");
    let prereleases = [
        "1.0.0-0",
        "1.0.0-zzz",
        "1.0.0-rc.99999",
        "1.0.0-alpha.1.2.3",
    ]
    .map(npm);

    // Act
    let actual = prereleases.each_ref().map(|pre| pre.precedence(&release));

    // Assert
    assert_eq!(actual, [Ordering::Less; 4]);
}

#[test]
fn numeric_prerelease_identifier_is_less_than_alphanumeric_one() {
    // Arrange
    let (numeric, alpha) = (npm("1.0.0-99999"), npm("1.0.0-a"));

    // Act
    let ordering = numeric.precedence(&alpha);

    // Assert
    assert_eq!(ordering, Ordering::Less);
}

#[test]
fn longer_prerelease_list_is_greater_when_prefix_is_equal() {
    // Arrange
    let (short, long) = (npm("1.0.0-alpha"), npm("1.0.0-alpha.0"));

    // Act
    let ordering = short.precedence(&long);

    // Assert
    assert_eq!(ordering, Ordering::Less);
}

#[test]
fn numeric_prerelease_identifiers_compare_by_value() {
    // Arrange
    let (nine, ten) = (npm("1.0.0-alpha.9"), npm("1.0.0-alpha.10"));

    // Act
    let ordering = nine.precedence(&ten);

    // Assert
    assert_eq!(ordering, Ordering::Less);
}
