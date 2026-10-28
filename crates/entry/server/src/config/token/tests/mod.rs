#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use crate::config::tests::{error_of, parse, with};

#[test]
fn token_principal_defaults_to_crane_robot_and_can_be_overridden() {
    // Arrange
    let cases = [with(&[]), with(&[(TOKEN_PRINCIPAL, " ci-bot ")])];

    // Act
    let principals = cases.map(|pairs| parse(&pairs).unwrap().principal);

    // Assert
    let robot = |id: &str| Principal {
        id: id.to_owned(),
        kind: PrincipalKind::Robot,
    };
    assert_eq!(principals, [robot("crane"), robot("ci-bot")]);
}

#[test]
fn blank_or_control_character_principal_is_rejected() {
    // Arrange
    let cases = [
        with(&[(TOKEN_PRINCIPAL, "  ")]),
        with(&[(TOKEN_PRINCIPAL, "a\nb")]),
    ];

    // Act
    let errors = cases.map(|pairs| error_of(&pairs));

    // Assert
    let invalid = || ConfigError::Invalid {
        var: TOKEN_PRINCIPAL,
        reason: "ожидалось непустое имя без управляющих символов",
    };
    assert_eq!(errors, [invalid(), invalid()]);
}
