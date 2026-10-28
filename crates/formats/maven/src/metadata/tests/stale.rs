use std::num::NonZeroUsize;

use crane_domain::registry::{Ecosystem, PackageVersion};

use super::super::stale_builds;
use super::fixtures::{HOUR, T0, package, raws, released};
use crate::ecosystem::MAVEN;

fn keep(count: usize) -> NonZeroUsize {
    NonZeroUsize::new(count).unwrap()
}

fn stale(base: &str, served: &[PackageVersion], count: usize) -> Vec<String> {
    raws(&stale_builds(&package(), base, served, keep(count)))
}

/// Четыре сборки базы `1.0` вперемешку (по времени, а не по списку или публикации),
/// релиз той же базы и сборки соседней базы.
fn mixed() -> Vec<PackageVersion> {
    vec![
        released("1.0-20261028.130000-2", T0 + 5 * HOUR, &[]),
        released("1.0", T0 + HOUR, &[]),
        released("1.0-20261028.120000-9", T0, &[]),
        released("1.0-20261028.130000-10", T0 + 4 * HOUR, &[]),
        released("1.1-20261028.100000-1", T0 - HOUR, &[]),
        released("1.0-20261027.230000-1", T0 + 9 * HOUR, &[]),
        released("1.0-SNAPSHOT", T0, &[]),
        released("0.9", T0, &[]),
    ]
}

#[test]
fn builds_beyond_the_newest_keep_are_stale_oldest_first() {
    // Arrange
    let served = mixed();

    // Act
    let actual = stale("1.0", &served, 2);

    // Assert
    assert_eq!(actual, ["1.0-20261027.230000-1", "1.0-20261028.120000-9"]);
}

#[test]
fn nothing_is_stale_while_builds_fit_into_keep() {
    // Arrange
    let served = mixed();

    // Act
    let actual = [3, 4, 100].map(|count| stale("1.0", &served, count));

    // Assert
    assert_eq!(actual, [vec!["1.0-20261027.230000-1"], vec![], vec![]]);
}

#[test]
fn releases_and_other_bases_are_never_stale_for_any_keep() {
    // Arrange
    let served = mixed();
    let protected = ["1.0", "1.0-SNAPSHOT", "0.9", "1.1-20261028.100000-1"];

    // Act
    let leaked: Vec<String> = (1..=8)
        .flat_map(|count| stale("1.0", &served, count))
        .filter(|raw| protected.contains(&raw.as_str()))
        .collect();

    // Assert
    assert_eq!(leaked, Vec::<String>::new());
}

#[test]
fn a_release_is_untouched_even_when_it_is_the_only_version() {
    // Arrange
    let served = [released("1.0", T0, &[]), released("1.0", T0 + HOUR, &[])];

    // Act
    let actual = stale("1.0", &served, 1);

    // Assert
    assert_eq!(actual, Vec::<String>::new());
}

#[test]
fn builds_of_other_packages_are_never_stale() {
    // Arrange
    let mut foreign = released("1.0-20261028.110000-1", T0 - HOUR, &[]);
    foreign.package = MAVEN.parse_name("com.acme:other").unwrap();
    let served = [
        foreign,
        released("1.0-20261028.120000-1", T0, &[]),
        released("1.0-20261028.130000-2", T0 + HOUR, &[]),
    ];

    // Act
    let actual = stale("1.0", &served, 1);

    // Assert
    assert_eq!(actual, ["1.0-20261028.120000-1"]);
}
