use crane_domain::registry::Ecosystem;

use crate::ecosystem::NPM;

#[test]
fn normalize_keeps_case_and_scope() {
    // Arrange
    let raws = ["@Acme/Ui", "JSONStream"];

    // Act
    let actual = raws.map(|raw| NPM.normalize(raw));

    // Assert
    assert_eq!(actual, ["@Acme/Ui", "JSONStream"]);
}
