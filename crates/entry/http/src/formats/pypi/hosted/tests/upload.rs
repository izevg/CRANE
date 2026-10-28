use super::*;

#[test]
fn text_budget_allows_exactly_the_limits_and_refuses_the_next_field() {
    // Arrange
    let mut by_count = TextBudget::default();
    let mut by_bytes = TextBudget::default();

    // Act
    let actual = (
        (0..TEXT_FIELD_COUNT).all(|_| by_count.take(1)),
        by_count.take(1),
        by_bytes.take(TEXT_TOTAL_LIMIT),
        by_bytes.take(1),
    );

    // Assert
    assert_eq!(actual, (true, false, true, false));
}
