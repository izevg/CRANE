use std::cmp::Ordering;

use crane_domain::registry::Ecosystem;

use super::oracle;
use super::table::{PAIRS, SEQUENCES};
use crate::ecosystem::MAVEN;

/// Составные части для сгенерированных версий: числа с нулями и без, квалификаторы в разных
/// регистрах, одиночные `a`/`b`/`m`, неизвестные строки, `min`/`max` и число длиннее `u64`.
const PIECES: &[&str] = &[
    "0",
    "1",
    "2",
    "3",
    "9",
    "10",
    "00",
    "007",
    "1234567890123456789012",
    "alpha",
    "beta",
    "BETA",
    "a",
    "b",
    "m",
    "milestone",
    "rc",
    "CR",
    "snapshot",
    "SNAPSHOT",
    "ga",
    "final",
    "release",
    "sp",
    "SP",
    "foo",
    "bar",
    "xyz",
    "min",
    "max",
];
/// Пустой разделитель склеивает токены разных родов (`a1`, `1a`) и сливает токены одного рода.
const JOINERS: &[&str] = &[".", "-", "_", "", ".", "-"];
const MAX_PIECES: u64 = 6;
const GENERATED: usize = 250;
const SEED: u64 = 0x5EED_CAFE;

/// Детерминированный xorshift64*: генератор без зависимостей.
struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) % bound
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[usize::try_from(self.below(items.len() as u64)).unwrap()]
    }
}

fn generated_versions(count: usize, seed: u64) -> Vec<String> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|_| {
            let pieces = 1 + rng.below(MAX_PIECES);
            let mut version = rng.pick(PIECES).to_owned();
            for _ in 1..pieces {
                version.push_str(rng.pick(JOINERS));
                version.push_str(rng.pick(PIECES));
            }
            version
        })
        .collect()
}

fn key_cmp(a: &str, b: &str) -> Ordering {
    let key = |raw: &str| MAVEN.parse_version(raw).unwrap().order_key().clone();
    key(a).cmp(&key(b))
}

/// Пары `(a, b, ожидаемое)`, на которых `actual` расходится с ожиданием.
fn mismatches<'a>(
    pairs: impl Iterator<Item = (&'a str, &'a str, Ordering)>,
    actual: impl Fn(&str, &str) -> Ordering,
) -> Vec<(&'a str, &'a str, Ordering)> {
    pairs
        .filter(|(a, b, expected)| actual(a, b) != *expected)
        .collect()
}

fn sequence_pairs() -> Vec<(&'static str, &'static str, Ordering)> {
    SEQUENCES
        .iter()
        .flat_map(|chain| {
            chain.iter().enumerate().flat_map(move |(i, a)| {
                chain[i + 1..].iter().map(move |b| (*a, *b, Ordering::Less))
            })
        })
        .collect()
}

fn table_pairs() -> impl Iterator<Item = (&'static str, &'static str, Ordering)> {
    PAIRS.iter().map(|(a, expected, b)| (*a, *b, *expected))
}

#[test]
fn oracle_port_agrees_with_resolver_unit_tests() {
    // Arrange
    let pairs: Vec<_> = table_pairs().chain(sequence_pairs()).collect();

    // Act
    let wrong = mismatches(pairs.iter().copied(), oracle::compare);

    // Assert
    assert_eq!(wrong, vec![]);
}

#[test]
fn key_order_agrees_with_resolver_unit_test_pairs() {
    // Arrange
    // Пустая версия вне грамматики Maven: `GenericVersion` читает её как «0».
    let pairs: Vec<_> = table_pairs()
        .filter(|(a, b, _)| !a.is_empty() && !b.is_empty())
        .collect();

    // Act
    let wrong = mismatches(pairs.iter().copied(), key_cmp);

    // Assert
    assert_eq!(wrong, vec![]);
}

#[test]
fn key_order_agrees_with_resolver_unit_test_sequences() {
    // Arrange
    let pairs = sequence_pairs();

    // Act
    let wrong = mismatches(pairs.iter().copied(), key_cmp);

    // Assert
    assert_eq!(wrong, vec![]);
}

#[test]
fn key_order_agrees_with_oracle_on_generated_versions() {
    // Arrange
    let versions = generated_versions(GENERATED, SEED);
    let pairs: Vec<(&str, &str, Ordering)> = versions
        .iter()
        .flat_map(|a| {
            versions
                .iter()
                .map(move |b| (a.as_str(), b.as_str(), oracle::compare(a, b)))
        })
        .collect();

    // Act
    let wrong = mismatches(pairs.iter().copied(), key_cmp);

    // Assert
    assert_eq!(wrong, vec![]);
}

#[test]
fn generated_versions_cover_every_ordering_outcome() {
    // Arrange
    let versions = generated_versions(GENERATED, SEED);

    // Act
    let outcomes: Vec<Ordering> = versions
        .iter()
        .flat_map(|a| versions.iter().map(move |b| oracle::compare(a, b)))
        .collect();

    // Assert
    let seen = [Ordering::Less, Ordering::Equal, Ordering::Greater]
        .map(|wanted| outcomes.iter().filter(|o| **o == wanted).count() > GENERATED);
    assert_eq!(seen, [true, true, true]);
}

#[test]
fn branch_versions_sort_before_numeric_ones_by_key() {
    // Arrange
    // Список из `GenericVersionTest.testSortBranchAndNumericVersions` (MRESOLVER-686).
    let mut versions = vec![
        "4.225.2.1-dev",
        "0-latest-SNAPSHOT",
        "foo-branch-1-SNAPSHOT",
        "0.5641-SNAPSHOT",
        "bar-branch-2-SNAPSHOT",
        "1.163.0.0-qa",
        "baz-branch-3-SNAPSHOT",
        "0.0-SNAPSHOT",
        "qux-branch-4-SNAPSHOT",
        "2.19.0.0-dev",
        "0.5640-SNAPSHOT",
        "3.59.0.0-dev",
    ];

    // Act
    versions.sort_by(|a, b| key_cmp(a, b));

    // Assert
    let branches = versions
        .iter()
        .take_while(|v| v.contains("-branch-"))
        .count();
    assert_eq!(
        (versions.first(), versions.last(), branches),
        (Some(&"bar-branch-2-SNAPSHOT"), Some(&"4.225.2.1-dev"), 4)
    );
}
