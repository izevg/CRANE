use std::time::{Duration, SystemTime};

use super::super::render::timestamp;
use super::fixtures::at;

#[test]
fn timestamp_is_utc_with_leap_days_and_century_rules() {
    // Arrange
    let cases = [
        (0, "19700101000000"),
        (951_782_400, "20000229000000"),
        (1_709_164_799, "20240228235959"),
        (1_709_251_200, "20240301000000"),
        (1_793_188_800, "20261028120000"),
        (4_102_444_799, "20991231235959"),
        (4_107_542_399, "21000228235959"),
        (4_107_542_400, "21000301000000"),
    ];

    // Act
    let actual = cases.map(|(seconds, _)| timestamp(at(seconds)));

    // Assert
    assert_eq!(actual, cases.map(|(_, expected)| expected));
}

#[test]
fn time_before_the_epoch_is_rendered_as_the_epoch() {
    // Arrange
    let before = SystemTime::UNIX_EPOCH - Duration::from_secs(1);

    // Act
    let rendered = timestamp(before);

    // Assert
    assert_eq!(rendered, "19700101000000");
}
