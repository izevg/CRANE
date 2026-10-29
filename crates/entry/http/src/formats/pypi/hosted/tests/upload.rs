use super::*;

// Пределы записаны литералами: тест, читающий константы, не заметил бы их порчи.
const FOUR_MIB: usize = 4_194_304;
const MAX_FIELDS: usize = 256;

#[test]
fn text_budget_allows_exactly_256_fields_and_refuses_the_next() {
    // Arrange
    let mut budget = TextBudget::default();

    // Act
    let actual = ((0..MAX_FIELDS).all(|_| budget.take(1)), budget.take(1));

    // Assert
    assert_eq!(actual, (true, false));
}

#[test]
fn text_budget_allows_exactly_four_mebibytes_and_refuses_one_byte_more() {
    // Arrange
    let (mut at_limit, mut over_limit) = (TextBudget::default(), TextBudget::default());

    // Act
    let actual = (at_limit.take(FOUR_MIB), over_limit.take(FOUR_MIB + 1));

    // Assert
    assert_eq!(actual, (true, false));
}

#[test]
fn text_budget_counts_the_field_name_together_with_the_value() {
    // Arrange
    let mut budget = TextBudget::default();
    let name = "n".repeat(FOUR_MIB);

    // Act
    let accepted = budget.take_field(&name, "x");

    // Assert
    assert!(!accepted);
}
