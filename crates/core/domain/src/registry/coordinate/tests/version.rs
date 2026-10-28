use std::collections::hash_map::DefaultHasher;

use super::*;
use crate::registry::OrderKeyBuilder;

fn version(canonical: &str, order: u64) -> Version {
    Version::from_parts(
        canonical.to_owned(),
        canonical.to_owned(),
        OrderKeyBuilder::new().number(order).finish(),
        false,
    )
}

fn hash_of(v: &Version) -> u64 {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn ord_breaks_ties_of_equal_order_by_canonical_while_precedence_stays_equal() {
    // Arrange
    let (a, b) = (version("1", 7), version("1.0", 7));

    // Act
    let actual = (a.cmp(&b), a.precedence(&b));

    // Assert
    assert_eq!(actual, (Ordering::Less, Ordering::Equal));
}

#[test]
fn precedence_follows_order_key_not_canonical() {
    // Arrange
    let (low, high) = (version("9", 1), version("10", 2));
    let (low_text, high_text) = (version("b", 1), version("a", 2));

    // Act
    let actual = (low.precedence(&high), low_text.precedence(&high_text));

    // Assert
    assert_eq!(actual, (Ordering::Less, Ordering::Less));
}

#[test]
fn eq_ord_and_hash_agree_on_a_table_of_pairs() {
    // Arrange
    let pairs = [
        (version("1", 1), version("1", 1)),
        (version("1", 1), version("2", 2)),
        (version("1", 7), version("1.0", 7)),
        (version("2", 2), version("1", 1)),
    ];

    // Act
    let actual = pairs.each_ref().map(|(a, b)| {
        let equal = a == b;
        (
            equal,
            a.cmp(b) == Ordering::Equal,
            equal.then(|| hash_of(a) == hash_of(b)),
        )
    });

    // Assert
    assert_eq!(
        actual,
        [
            (true, true, Some(true)),
            (false, false, None),
            (false, false, None),
            (false, false, None),
        ]
    );
}

#[test]
fn versions_with_equal_canonical_are_equal_regardless_of_written_form() {
    // Arrange
    let a = Version::from_parts(
        "1.0.0".into(),
        "1.0.0+a".into(),
        OrderKeyBuilder::new().finish(),
        false,
    );
    let b = Version::from_parts(
        "1.0.0".into(),
        "1.0.0+b".into(),
        OrderKeyBuilder::new().finish(),
        false,
    );

    // Act
    let actual = (a == b, hash_of(&a) == hash_of(&b), a.cmp(&b));

    // Assert
    assert_eq!(actual, (true, true, Ordering::Equal));
}
