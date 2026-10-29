use crane_domain::registry::{Ecosystem, NameError};

use crate::ecosystem::PYPI;

#[test]
fn prefix_base_folds_runs_of_separators_and_drops_the_boundary_one() {
    // Arrange
    let raws = ["Acme.Pay__-Core", "acme-_", "-"];

    // Act
    let actual = raws.map(|raw| PYPI.prefix_base(raw));

    // Assert
    assert_eq!(
        actual,
        [
            Ok("acme-pay-core".to_owned()),
            Ok("acme".to_owned()),
            Err(NameError::Empty)
        ]
    );
}
