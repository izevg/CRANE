use super::super::builds;
use super::fixtures::{HOUR, MINUTE, T0, package, released};

const OLD: &str = "1.2-20261028.120000-1";
const MID: &str = "1.2-20261028.130000-2";
const NEW: &str = "1.2-20261028.140000-10";

fn served() -> [crane_domain::registry::PackageVersion; 3] {
    [
        released(
            OLD,
            T0,
            &[
                "core-1.2-20261028.120000-1.jar",
                "core-1.2-20261028.120000-1.pom",
                "core-1.2-20261028.120000-1-sources.jar",
            ],
        ),
        released(
            MID,
            T0 + HOUR,
            &[
                "core-1.2-20261028.130000-2.jar",
                "core-1.2-20261028.130000-2.pom",
            ],
        ),
        released(
            NEW,
            T0 + 2 * HOUR + 5 * MINUTE,
            &[
                "core-1.2-20261028.140000-10.jar",
                "core-1.2-20261028.140000-10.pom",
                "core-1.2-20261028.140000-10.jar.asc",
            ],
        ),
    ]
}

const EXPECTED: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<metadata modelVersion=\"1.1.0\">\n",
    "  <groupId>com.acme</groupId>\n",
    "  <artifactId>core</artifactId>\n",
    "  <version>1.2-SNAPSHOT</version>\n",
    "  <versioning>\n",
    "    <snapshot>\n",
    "      <timestamp>20261028.140000</timestamp>\n",
    "      <buildNumber>10</buildNumber>\n",
    "    </snapshot>\n",
    "    <lastUpdated>20261028140500</lastUpdated>\n",
    "    <snapshotVersions>\n",
    "      <snapshotVersion>\n",
    "        <extension>jar</extension>\n",
    "        <value>1.2-20261028.140000-10</value>\n",
    "        <updated>20261028140500</updated>\n",
    "      </snapshotVersion>\n",
    "      <snapshotVersion>\n",
    "        <extension>jar.asc</extension>\n",
    "        <value>1.2-20261028.140000-10</value>\n",
    "        <updated>20261028140500</updated>\n",
    "      </snapshotVersion>\n",
    "      <snapshotVersion>\n",
    "        <extension>pom</extension>\n",
    "        <value>1.2-20261028.140000-10</value>\n",
    "        <updated>20261028140500</updated>\n",
    "      </snapshotVersion>\n",
    "      <snapshotVersion>\n",
    "        <classifier>sources</classifier>\n",
    "        <extension>jar</extension>\n",
    "        <value>1.2-20261028.120000-1</value>\n",
    "        <updated>20261028120000</updated>\n",
    "      </snapshotVersion>\n",
    "    </snapshotVersions>\n",
    "  </versioning>\n",
    "</metadata>\n",
);

#[test]
fn snapshot_level_document_points_every_file_at_its_newest_build() {
    // Arrange
    let served = served();

    // Act
    let xml = builds(&package(), "1.2", &served);

    // Assert
    assert_eq!(xml.as_deref(), Some(EXPECTED));
}

#[test]
fn document_does_not_depend_on_the_order_of_the_catalog_listing() {
    // Arrange
    let [old, mid, new] = served();
    let shuffled = [mid, new, old];

    // Act
    let xml = builds(&package(), "1.2", &shuffled);

    // Assert
    assert_eq!(xml.as_deref(), Some(EXPECTED));
}

#[test]
fn builds_are_ordered_by_number_not_by_text() {
    // Arrange
    let served = [
        released(
            "1.2-20261028.120000-9",
            T0,
            &["core-1.2-20261028.120000-9.jar"],
        ),
        released(
            "1.2-20261028.120000-10",
            T0 + MINUTE,
            &["core-1.2-20261028.120000-10.jar"],
        ),
    ];

    // Act
    let xml = builds(&package(), "1.2", &served).unwrap();

    // Assert
    assert!(xml.contains("<buildNumber>10</buildNumber>"));
}

#[test]
fn base_without_builds_has_no_document() {
    // Arrange
    let served = [
        released("1.2", T0, &["core-1.2.jar"]),
        released(OLD, T0, &[]),
    ];

    // Act
    let xml = builds(&package(), "1.3", &served);

    // Assert
    assert_eq!(xml, None);
}

#[test]
fn files_that_do_not_belong_to_their_build_stay_out_of_the_projection() {
    // Arrange
    let served = [released(
        OLD,
        T0,
        &[
            "core-1.2-20261028.120000-1.jar",
            "core-1.2-20261028.999999-7.pom",
            "core-1.2-SNAPSHOT.zip",
            "unrelated.txt",
        ],
    )];

    // Act
    let xml = builds(&package(), "1.2", &served).unwrap();

    // Assert
    let values: Vec<&str> = xml
        .lines()
        .filter(|l| l.contains("<extension>"))
        .map(str::trim)
        .collect();
    assert_eq!(values, ["<extension>jar</extension>"]);
}
