use crane_domain::registry::{Ecosystem, NameError};

use crate::ecosystem::CARGO;

#[test]
fn prefix_base_is_validated_and_folded_like_a_name() {
    // Arrange
    let raws = ["Acme-Pay_Core", "ACME", "1acme", "ac me"];

    // Act
    let actual = raws.map(|raw| CARGO.prefix_base(raw));

    // Assert
    assert_eq!(
        actual,
        [
            Ok("acme_pay_core".to_owned()),
            Ok("acme".to_owned()),
            Err(NameError::InvalidStart('1')),
            Err(NameError::InvalidChar(' ')),
        ]
    );
}
