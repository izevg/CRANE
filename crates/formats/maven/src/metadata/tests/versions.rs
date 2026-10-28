use crane_domain::registry::{OrderKeyBuilder, PackageVersion, Version, VersionState};

use super::super::versions;
use super::fixtures::{HOUR, MINUTE, T0, package, released, with_version};

const BUILD_1: &str = "1.2-20261028.120000-1";
const BUILD_2: &str = "1.2-20261028.130000-2";

#[test]
fn artifact_level_document_lists_bases_with_latest_and_release() {
    // Arrange
    let served = [
        released(BUILD_2, T0 + HOUR, &[]),
        released("1.1", T0 - HOUR, &[]),
        released(BUILD_1, T0, &[]),
        released("1.0", T0 - 2 * HOUR, &[]),
    ];

    // Act
    let xml = versions(&package(), &served);

    // Assert
    assert_eq!(
        xml.as_deref(),
        Some(concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<metadata modelVersion=\"1.1.0\">\n",
            "  <groupId>com.acme</groupId>\n",
            "  <artifactId>core</artifactId>\n",
            "  <versioning>\n",
            "    <latest>1.2-SNAPSHOT</latest>\n",
            "    <release>1.1</release>\n",
            "    <versions>\n",
            "      <version>1.0</version>\n",
            "      <version>1.1</version>\n",
            "      <version>1.2-SNAPSHOT</version>\n",
            "    </versions>\n",
            "    <lastUpdated>20261028130000</lastUpdated>\n",
            "  </versioning>\n",
            "</metadata>\n",
        ))
    );
}

#[test]
fn release_and_latest_follow_version_order_not_publication_time() {
    // Arrange
    let served = [
        released("2.0", T0 - HOUR, &[]),
        released("1.0", T0, &[]),
        released("1.9-SNAPSHOT", T0 + HOUR, &[]),
    ];

    // Act
    let xml = versions(&package(), &served).unwrap();

    // Assert
    let tags: Vec<&str> = xml
        .lines()
        .filter(|l| l.contains("<latest>") || l.contains("<release>"))
        .map(str::trim)
        .collect();
    assert_eq!(tags, ["<latest>2.0</latest>", "<release>2.0</release>"]);
}

#[test]
fn release_element_is_absent_when_only_builds_exist() {
    // Arrange
    let served = [
        released(BUILD_1, T0, &[]),
        released(BUILD_2, T0 + HOUR, &[]),
    ];

    // Act
    let xml = versions(&package(), &served).unwrap();

    // Assert
    assert_eq!(
        (
            xml.contains("<release>"),
            xml.contains("<latest>1.2-SNAPSHOT</latest>")
        ),
        (false, true)
    );
}

#[test]
fn builds_of_one_base_are_listed_once() {
    // Arrange
    let served = [
        released(BUILD_1, T0, &[]),
        released(BUILD_2, T0 + HOUR, &[]),
    ];

    // Act
    let xml = versions(&package(), &served).unwrap();

    // Assert
    assert_eq!(xml.matches("<version>1.2-SNAPSHOT</version>").count(), 1);
}

#[test]
fn empty_listing_means_no_document() {
    // Arrange
    let served: [PackageVersion; 0] = [];

    // Act
    let xml = versions(&package(), &served);

    // Assert
    assert_eq!(xml, None);
}

#[test]
fn last_updated_is_the_newest_publication_to_the_second() {
    // Arrange
    let served = [
        released("1.0", T0 + 5 * MINUTE + 7, &[]),
        released("0.9", T0 + HOUR, &[]),
    ];

    // Act
    let xml = versions(&package(), &served).unwrap();

    // Assert
    assert!(xml.contains("<lastUpdated>20261028130000</lastUpdated>"));
}

#[test]
fn special_characters_are_escaped_in_text() {
    // Arrange
    let raw = "1<2>&\"'";
    let odd = with_version(
        Version::from_parts(
            raw.to_owned(),
            raw.to_owned(),
            OrderKeyBuilder::new().rank(1).finish(),
            false,
        ),
        VersionState::Released,
        T0,
        &[],
    );

    // Act
    let xml = versions(&package(), &[odd]).unwrap();

    // Assert
    assert!(xml.contains("<version>1&lt;2&gt;&amp;&quot;&apos;</version>"));
}

#[test]
fn literal_snapshot_never_becomes_the_release() {
    // Arrange
    let served = [
        released("0.9", T0, &[]),
        released("1.0-SNAPSHOT", T0 + HOUR, &[]),
    ];

    // Act
    let xml = versions(&package(), &served).unwrap();

    // Assert
    let tags: Vec<&str> = xml
        .lines()
        .filter(|l| l.contains("<latest>") || l.contains("<release>"))
        .map(str::trim)
        .collect();
    assert_eq!(
        tags,
        ["<latest>1.0-SNAPSHOT</latest>", "<release>0.9</release>"]
    );
}

#[test]
fn bases_with_equal_keys_are_listed_once_each_in_order_of_appearance() {
    // Arrange
    // `1-SNAPSHOT` и `1.0-SNAPSHOT` равны по порядку, поэтому их сборки перемежаются по времени.
    let served = [
        released("1-20261028.120000-1", T0, &[]),
        released("1.0-20261028.130000-1", T0 + HOUR, &[]),
        released("1-20261028.140000-2", T0 + 2 * HOUR, &[]),
    ];

    // Act
    let xml = versions(&package(), &served).unwrap();

    // Assert
    let listed: Vec<&str> = xml
        .lines()
        .filter(|l| l.contains("<version>"))
        .map(str::trim)
        .collect();
    assert_eq!(
        listed,
        [
            "<version>1-SNAPSHOT</version>",
            "<version>1.0-SNAPSHOT</version>"
        ]
    );
}
