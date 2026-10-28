use std::cmp::Ordering;

use super::*;

fn numbers(values: &[u64]) -> OrderKey {
    values
        .iter()
        .fold(OrderKeyBuilder::new(), |b, v| b.number(*v))
        .finish()
}

fn text(value: &str) -> OrderKey {
    OrderKeyBuilder::new().text(value).finish()
}

#[test]
fn numbers_compare_by_value_not_by_bytes_length_or_digits() {
    // Arrange
    let pairs = [
        (numbers(&[0]), numbers(&[1])),
        (numbers(&[9]), numbers(&[10])),
        (numbers(&[255]), numbers(&[256])),
        (numbers(&[256]), numbers(&[257])),
        (
            numbers(&[u64::from(u32::MAX)]),
            numbers(&[u64::from(u32::MAX) + 1]),
        ),
        (numbers(&[u64::MAX - 1]), numbers(&[u64::MAX])),
        (numbers(&[2]), numbers(&[2])),
        (numbers(&[10]), numbers(&[9])),
    ];

    // Act
    let actual = pairs.map(|(a, b)| a.cmp(&b));

    // Assert
    assert_eq!(
        actual,
        [
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Equal,
            Ordering::Greater,
        ]
    );
}

#[test]
fn number_sequence_is_smaller_than_its_continuation_and_decided_by_first_difference() {
    // Arrange
    let pairs = [
        (numbers(&[1]), numbers(&[1, 0])),
        (numbers(&[1, 9]), numbers(&[1, 10])),
        (numbers(&[1, 255]), numbers(&[2, 0])),
        (numbers(&[2]), numbers(&[1, 99])),
    ];

    // Act
    let actual = pairs.map(|(a, b)| a.cmp(&b));

    // Assert
    assert_eq!(
        actual,
        [
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Greater
        ]
    );
}

#[test]
fn texts_compare_as_strings_with_prefix_smaller_than_continuation() {
    // Arrange
    let pairs = [
        (text(""), text("a")),
        (text("a"), text("ab")),
        (text("ab"), text("b")),
        (text("a"), text("a\0")),
        (text("a\0"), text("a\u{1}")),
        (text("a\0"), text("a\0\0")),
        (text("a\0b"), text("a\0b")),
        (text("b"), text("ab")),
    ];

    // Act
    let actual = pairs.map(|(a, b)| a.cmp(&b));

    // Assert
    assert_eq!(
        actual,
        [
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Equal,
            Ordering::Greater,
        ]
    );
}

#[test]
fn text_terminator_keeps_following_elements_from_leaking_into_the_comparison() {
    // Arrange
    let pairs = [
        (
            OrderKeyBuilder::new().text("a").text("z").finish(),
            OrderKeyBuilder::new().text("ab").text("a").finish(),
        ),
        (
            OrderKeyBuilder::new().text("a").number(2).finish(),
            OrderKeyBuilder::new().text("a").number(10).finish(),
        ),
    ];

    // Act
    let actual = pairs.map(|(a, b)| a.cmp(&b));

    // Assert
    assert_eq!(actual, [Ordering::Less, Ordering::Less]);
}

#[test]
fn rank_orders_variants_before_the_rest_of_the_key() {
    // Arrange
    let pre = OrderKeyBuilder::new().number(1).rank(0).text("rc").finish();
    let release = OrderKeyBuilder::new().number(1).rank(1).finish();

    // Act
    let ordering = pre.cmp(&release);

    // Assert
    assert_eq!(ordering, Ordering::Less);
}

fn digits(value: &str) -> OrderKey {
    OrderKeyBuilder::new().digits(value).finish()
}

#[test]
fn digits_compare_by_value_whatever_their_length() {
    // Arrange
    let pairs = [
        (digits("9"), digits("10")),
        (digits("2"), digits("100")),
        (
            digits("99999999999999999999"),
            digits("100000000000000000000"),
        ),
        (digits("0"), digits("1")),
        (digits("007"), digits("7")),
        (digits("10"), digits("9")),
    ];

    // Act
    let actual = pairs.map(|(a, b)| a.cmp(&b));

    // Assert
    assert_eq!(
        actual,
        [
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Less,
            Ordering::Equal,
            Ordering::Greater,
        ]
    );
}

#[test]
fn digits_keep_following_elements_out_of_the_comparison() {
    // Arrange
    let (short, long) = (
        OrderKeyBuilder::new().digits("9").text("z").finish(),
        OrderKeyBuilder::new().digits("10").text("a").finish(),
    );

    // Act
    let ordering = short.cmp(&long);

    // Assert
    assert_eq!(ordering, Ordering::Less);
}

#[test]
fn encoding_is_stable_because_keys_are_persisted() {
    // Arrange
    let keys = [
        OrderKeyBuilder::new().number(0).finish(),
        OrderKeyBuilder::new().number(1).finish(),
        OrderKeyBuilder::new().number(256).finish(),
        OrderKeyBuilder::new().text("a\0").finish(),
        OrderKeyBuilder::new().rank(7).digits("007").finish(),
    ];

    // Act
    let actual = keys.each_ref().map(OrderKey::as_bytes);

    // Assert
    let expected: [&[u8]; 5] = [
        &[0],
        &[1, 1],
        &[2, 1, 0],
        &[0x61, 0, 0xFF, 0, 1],
        &[7, 1, 1, b'7'],
    ];
    assert_eq!(actual, expected);
}
