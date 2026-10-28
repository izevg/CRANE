use std::cmp::Ordering;

use crane_domain::registry::{Ecosystem, VersionError};

use crate::ecosystem::{Build, MAVEN};

fn sorted_by_key<'a>(raws: &[&'a str]) -> Vec<&'a str> {
    let mut sorted = raws.to_vec();
    sorted.sort_by_key(|raw| MAVEN.parse_version(raw).unwrap().order_key().clone());
    sorted
}

#[test]
fn canonical_form_equals_written_form() {
    // Arrange
    let long_digits = "1.1234567890123456789012345678901";
    let raws = [
        "1.0",
        "1.0-SNAPSHOT",
        "1.0-20261028.123456-3",
        "2.0.0.Final",
        "1.0+build.5",
        "1.0~rc1",
        "v1",
        "1.0_01",
        "1.0'x&y",
        long_digits,
    ];

    // Act
    let actual = raws.map(|raw| {
        MAVEN
            .parse_version(raw)
            .map(|v| (v.canonical().to_owned(), v.as_written().to_owned()))
    });

    // Assert
    assert_eq!(actual, raws.map(|raw| Ok((raw.to_owned(), raw.to_owned()))));
}

#[test]
fn versions_unfit_for_a_path_segment_are_rejected() {
    // Arrange
    let too_long = "1".repeat(129);
    let raws = [
        "",
        ".",
        "..",
        "1/0",
        "1\\0",
        "1\u{0}",
        "1\n0",
        "1 0",
        "1:0",
        "1?0",
        "1*0",
        "1<0",
        "1>0",
        "1|0",
        "1\"0",
        "1%2e",
        "1#0",
        "1.0é",
        too_long.as_str(),
    ];

    // Act
    let actual = raws.map(|raw| MAVEN.parse_version(raw).is_err());

    // Assert
    assert_eq!(actual, raws.map(|_| true));
}

#[test]
fn version_of_maximum_length_is_accepted() {
    // Arrange
    let raw = "1".repeat(128);

    // Act
    let result = MAVEN.parse_version(&raw);

    // Assert
    assert_eq!(result.map(|v| v.canonical().len()), Ok(128));
}

#[test]
fn rejection_names_the_raw_version() {
    // Arrange
    let raw = "1/0";

    // Act
    let result = MAVEN.parse_version(raw);

    // Assert
    assert!(matches!(
        result,
        Err(VersionError::Invalid { raw: ref got, .. }) if got == raw
    ));
}

#[test]
fn only_timestamp_builds_are_prerelease() {
    // Arrange
    let raws = [
        "1.0-20261028.123456-3",
        "1.0-SNAPSHOT",
        "1.0-alpha-1",
        "1.0",
        "1.0-20261028.123456",
    ];

    // Act
    let actual = raws.map(|raw| MAVEN.parse_version(raw).unwrap().is_prerelease());

    // Assert
    assert_eq!(actual, [true, false, false, false, false]);
}

#[test]
fn build_name_is_split_into_base_stamp_and_number() {
    // Arrange
    let raws = [
        "1.0-20261028.123456-3",
        "1-0-20261028.012345-12",
        "2.0-beta-20260101.000001-4294967295",
    ];

    // Act
    let actual = raws.map(Build::parse);

    // Assert
    assert_eq!(
        actual,
        [
            Some(Build {
                base: "1.0",
                date: 20_261_028,
                time: 123_456,
                number: 3
            }),
            Some(Build {
                base: "1-0",
                date: 20_261_028,
                time: 12_345,
                number: 12
            }),
            Some(Build {
                base: "2.0-beta",
                date: 20_260_101,
                time: 1,
                number: u32::MAX
            }),
        ]
    );
}

#[test]
fn names_that_only_resemble_a_build_are_not_builds() {
    // Arrange
    let raws = [
        "1.0-SNAPSHOT",
        "1.0-20261028.123456",
        "-20261028.123456-3",
        "1.0-2026102.123456-3",
        "1.0-20261028.12345-3",
        "1.0-20261028.1234567-3",
        "1.0-20261028.123456-",
        "1.0-20261028.123456-+3",
        "1.0-20261028.123456-4294967296",
        "1.0-2026102a.123456-3",
        "1.0-20261028.123456-3-sources",
        "20261028.123456-3",
    ];

    // Act
    let actual = raws.map(Build::parse);

    // Assert
    assert_eq!(actual, raws.map(|_| None));
}

#[test]
fn build_renders_its_snapshot_directory_and_zero_padded_timestamp() {
    // Arrange
    let build = Build::parse("1.0-20261028.012345-3").unwrap();

    // Act
    let rendered = (build.base_version(), build.timestamp());

    // Assert
    assert_eq!(
        rendered,
        ("1.0-SNAPSHOT".to_owned(), "20261028.012345".to_owned())
    );
}

#[test]
fn builds_of_a_base_sort_by_timestamp_then_numeric_build_number() {
    // Arrange
    let raws = [
        "1.0-20261028.123456-10",
        "1.0-20261028.123500-1",
        "1.0-20261027.235959-9",
        "1.0-20261028.123456-2",
        "1.0-20261028.023456-7",
    ];

    // Act
    let sorted = sorted_by_key(&raws);

    // Assert
    assert_eq!(
        sorted,
        [
            "1.0-20261027.235959-9",
            "1.0-20261028.023456-7",
            "1.0-20261028.123456-2",
            "1.0-20261028.123456-10",
            "1.0-20261028.123500-1",
        ]
    );
}

#[test]
fn builds_sit_between_their_snapshot_and_the_release() {
    // Arrange
    let chain = [
        "1.0-rc-1",
        "1.0-SNAPSHOT",
        "1.0-20261027.235959-1",
        "1.0-20261028.123456-2",
        "1.0-SNAPSHOT.1",
        "1.0",
        "1.0.1-20261001.000000-1",
        "1.0.1-SNAPSHOT.1",
        "1.0.1",
        "1.1-20260101.000000-1",
    ];

    // Act
    let steps: Vec<Ordering> = chain
        .windows(2)
        .map(|pair| {
            let key = |raw: &str| MAVEN.parse_version(raw).unwrap().order_key().clone();
            key(pair[0]).cmp(&key(pair[1]))
        })
        .collect();

    // Assert
    assert_eq!(steps, vec![Ordering::Less; chain.len() - 1]);
}

#[test]
fn build_is_lower_than_release_of_any_equal_spelling() {
    // Arrange
    let releases = ["1", "1.0", "1.0.0", "1.0-ga", "1.0.0-final"];
    let build = MAVEN.parse_version("1.0-20261028.123456-3").unwrap();

    // Act
    let actual = releases.map(|raw| build.precedence(&MAVEN.parse_version(raw).unwrap()));

    // Assert
    assert_eq!(actual, releases.map(|_| Ordering::Less));
}
