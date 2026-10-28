use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crane_domain::registry::{Version, VersionError};

use crate::ecosystem::parse_version;

fn cargo(raw: &str) -> Version {
    parse_version(raw).unwrap()
}

fn hash_of(v: &Version) -> u64 {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn versions_follow_semver_precedence() {
    // Arrange
    let ordered = [
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-rc.1",
        "1.0.0",
        "1.0.1",
        "1.1.0",
    ];
    let mut shuffled: Vec<Version> = ordered.iter().rev().map(|r| cargo(r)).collect();

    // Act
    shuffled.sort();

    // Assert
    let result: Vec<&str> = shuffled.iter().map(Version::as_written).collect();
    assert_eq!(result, ordered);
}

#[test]
fn build_metadata_is_ignored_in_equality() {
    // Arrange
    let a = cargo("1.0.0+a");
    let b = cargo("1.0.0+b");

    // Act
    let equal = a == b;

    // Assert
    assert!(equal);
}

#[test]
fn build_metadata_is_ignored_in_hash() {
    // Arrange
    let a = cargo("1.0.0+a");
    let b = cargo("1.0.0+b");

    // Act
    let (ha, hb) = (hash_of(&a), hash_of(&b));

    // Assert
    assert_eq!(ha, hb);
}

#[test]
fn build_metadata_is_ignored_in_ordering() {
    // Arrange
    let a = cargo("1.0.0+z");
    let b = cargo("1.0.0+a");

    // Act
    let ordering = a.cmp(&b);

    // Assert
    assert_eq!(ordering, Ordering::Equal);
}

#[test]
fn canonical_form_drops_build_metadata() {
    // Arrange
    let versions = [cargo("1.0.0+a"), cargo("1.0.0-rc.1+x")];

    // Act
    let canonical: Vec<String> = versions.iter().map(|v| v.canonical().to_owned()).collect();

    // Assert
    assert_eq!(canonical, ["1.0.0", "1.0.0-rc.1"]);
}

#[test]
fn written_form_keeps_build_metadata() {
    // Arrange
    let version = cargo("1.0.0+build.5");

    // Act
    let (written, shown) = (version.as_written().to_owned(), version.to_string());

    // Assert
    assert_eq!(
        (written.as_str(), shown.as_str()),
        ("1.0.0+build.5", "1.0.0+build.5")
    );
}

#[test]
fn malformed_versions_are_rejected() {
    // Arrange
    let raws = ["1.0", "v1.0.0", "01.0.0", ""];

    // Act
    let errors: Vec<_> = raws.iter().map(|r| parse_version(r).unwrap_err()).collect();

    // Assert
    assert!(
        errors
            .iter()
            .zip(raws)
            .all(|(e, raw)| matches!(e, VersionError::Invalid { raw: r, .. } if r == raw))
    );
}

#[test]
fn versions_differing_in_patch_are_not_equal() {
    // Arrange
    let (a, b) = (cargo("1.0.0"), cargo("1.0.1"));

    // Act
    let equal = a == b;

    // Assert
    assert!(!equal);
}

#[test]
fn pre_release_is_not_equal_to_its_release() {
    // Arrange
    let (rc, release) = (cargo("1.0.0-rc.1"), cargo("1.0.0"));

    // Act
    let equal = rc == release;

    // Assert
    assert!(!equal);
}

#[test]
fn prerelease_is_detected_only_by_pre_part() {
    // Arrange
    let versions = ["1.0.0", "1.0.0+build-1", "1.0.0-rc.1", "1.0.0-rc.1+b"].map(cargo);

    // Act
    let actual = versions.each_ref().map(Version::is_prerelease);

    // Assert
    assert_eq!(actual, [false, false, true, true]);
}

/// Порядок из semver.org и граничные случаи идентификаторов, по возрастанию.
const SEMVER_CHAIN: [&str; 19] = [
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
        .map(|(a, b)| cargo(a).precedence(&cargo(b)))
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
    let release = cargo("1.0.0");
    let prereleases = [
        "1.0.0-0",
        "1.0.0-zzz",
        "1.0.0-rc.99999",
        "1.0.0-alpha.1.2.3",
    ]
    .map(cargo);

    // Act
    let actual = prereleases.each_ref().map(|pre| pre.precedence(&release));

    // Assert
    assert_eq!(actual, [Ordering::Less; 4]);
}

#[test]
fn numeric_prerelease_identifier_is_less_than_alphanumeric_one() {
    // Arrange
    let (numeric, alpha) = (cargo("1.0.0-99999"), cargo("1.0.0-a"));

    // Act
    let ordering = numeric.precedence(&alpha);

    // Assert
    assert_eq!(ordering, Ordering::Less);
}

#[test]
fn longer_prerelease_list_is_greater_when_prefix_is_equal() {
    // Arrange
    let (short, long) = (cargo("1.0.0-alpha"), cargo("1.0.0-alpha.0"));

    // Act
    let ordering = short.precedence(&long);

    // Assert
    assert_eq!(ordering, Ordering::Less);
}

#[test]
fn numeric_prerelease_identifiers_compare_by_value() {
    // Arrange
    let (nine, ten) = (cargo("1.0.0-alpha.9"), cargo("1.0.0-alpha.10"));

    // Act
    let ordering = nine.precedence(&ten);

    // Assert
    assert_eq!(ordering, Ordering::Less);
}
