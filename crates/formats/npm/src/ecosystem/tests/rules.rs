use crane_domain::registry::Ecosystem;

use crate::ecosystem::NPM;

#[test]
fn prefix_base_keeps_case() {
    // Arrange
    let raws = ["Acme", "JSONStream"];

    // Act
    let actual = raws.map(|raw| NPM.prefix_base(raw));

    // Assert
    assert_eq!(actual, [Ok("Acme".to_owned()), Ok("JSONStream".to_owned())]);
}
