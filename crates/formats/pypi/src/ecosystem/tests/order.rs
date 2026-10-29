//! Порядок версий: эталон — цепочка по возрастанию из набора тестов `packaging` и PEP 440.

use std::cmp::Ordering;

use crane_domain::registry::Version;

use crate::ecosystem::parse_version;

/// Строго по возрастанию: epoch 0 (pre/post/dev/local), затем следующие релизы и epoch 1.
const ASCENDING: [&str; 38] = [
    "0.9",
    "1.0.dev456",
    "1.0a1",
    "1.0a2.dev456",
    "1.0a12.dev456",
    "1.0a12",
    "1.0b1.dev456",
    "1.0b2",
    "1.0b2.post345.dev456",
    "1.0b2.post345",
    "1.0rc1.dev456",
    "1.0rc1",
    "1.0rc2",
    "1.0",
    "1.0+abc",
    "1.0+abc.5",
    "1.0+abc.7",
    "1.0+5",
    "1.0+10",
    "1.0.post456.dev34",
    "1.0.post456",
    "1.0.post456+local",
    "1.0.post1000",
    "1.0.1.dev1",
    "1.0.1",
    "1.1.dev1",
    "1.1",
    "1.2",
    "1.9",
    "1.10",
    "1.10.0.1",
    "2",
    "10",
    "100.0",
    "1!0.1.dev1",
    "1!0.1",
    "1!0.1.post1",
    "2!0",
];

#[test]
fn order_keys_sort_versions_exactly_as_the_reference_chain() {
    // Arrange
    let mut reversed: Vec<_> = ASCENDING
        .iter()
        .rev()
        .map(|raw| parse_version(raw).unwrap())
        .collect();

    // Act
    reversed.sort_by(|a, b| a.order_key().cmp(b.order_key()));

    // Assert
    let sorted: Vec<&str> = reversed.iter().map(Version::as_written).collect();
    assert_eq!(sorted, ASCENDING);
}

#[test]
fn order_keys_agree_with_pep440_rs_on_every_pair_of_the_chain() {
    // Arrange
    let versions: Vec<_> = ASCENDING
        .iter()
        .map(|raw| {
            (
                parse_version(raw).unwrap(),
                raw.parse::<pep440_rs::Version>().unwrap(),
            )
        })
        .collect();

    // Act
    let disagreeing: Vec<(&str, &str)> = versions
        .iter()
        .flat_map(|a| versions.iter().map(move |b| (a, b)))
        .filter(|((ka, ra), (kb, rb))| ka.order_key().cmp(kb.order_key()) != ra.cmp(rb))
        .map(|((ka, _), (kb, _))| (ka.as_written(), kb.as_written()))
        .collect();

    // Assert
    assert_eq!(disagreeing, Vec::<(&str, &str)>::new());
}

#[test]
fn only_dev_release_precedes_every_prerelease_of_the_same_release() {
    // Arrange
    let (dev, alpha) = (
        parse_version("2.0.dev9").unwrap(),
        parse_version("2.0a0").unwrap(),
    );

    // Act
    let order = dev.order_key().cmp(alpha.order_key());

    // Assert
    assert_eq!(order, Ordering::Less);
}
