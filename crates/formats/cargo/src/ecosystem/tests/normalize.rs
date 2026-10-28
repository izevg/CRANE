use crane_domain::registry::Ecosystem;

use crate::ecosystem::CARGO;

#[test]
fn normalize_folds_case_and_dashes() {
    // Arrange
    let raws = ["Acme-Pay_Core", "acme-", "ACME"];

    // Act
    let actual = raws.map(|raw| CARGO.normalize(raw));

    // Assert
    assert_eq!(actual, ["acme_pay_core", "acme_", "acme"]);
}
