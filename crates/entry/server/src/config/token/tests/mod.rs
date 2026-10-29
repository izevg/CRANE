#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::access::{BOOTSTRAP_PRINCIPAL_ID, SYSTEM_PRINCIPAL_ID};

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
fn principal_outside_the_grammar_or_reserved_is_rejected() {
    // Arrange
    let long = "a".repeat(65);
    let names = [
        SYSTEM_PRINCIPAL_ID,
        BOOTSTRAP_PRINCIPAL_ID,
        "cli:root",
        "crane:anything",
        "Crane",
        "-ci",
        ".ci",
        "ci bot",
        "ci\nbot",
        "ci/bot",
        "робот",
        "  ",
        &long,
    ];

    // Act
    let errors = names.map(|name| {
        let mut pairs: Vec<(&str, &str)> = with(&[]);
        pairs.push((TOKEN_PRINCIPAL, name));
        parse(&pairs).err()
    });

    // Assert
    let invalid = || {
        Some(ConfigError::Invalid {
            var: TOKEN_PRINCIPAL,
            reason: "ожидалось имя вида [a-z0-9][a-z0-9._-]{0,63}",
        })
    };
    assert_eq!(errors.to_vec(), vec![invalid(); names.len()]);
}

#[test]
fn principal_inside_the_grammar_is_accepted() {
    // Arrange
    let at_limit = "a".repeat(64);
    let names = ["crane", "ci-bot", "a.b_c", "7", &at_limit];

    // Act
    let accepted = names.map(|name| {
        let mut pairs: Vec<(&str, &str)> = with(&[]);
        pairs.push((TOKEN_PRINCIPAL, name));
        parse(&pairs).is_ok()
    });

    // Assert
    assert_eq!(accepted, [true; 5]);
}

#[test]
fn rejected_principal_error_does_not_contain_the_value() {
    // Arrange
    let mut pairs: Vec<(&str, &str)> = with(&[]);
    pairs.push((TOKEN_PRINCIPAL, "Secret-Name"));

    // Act
    let message = error_of(&pairs).to_string();

    // Assert
    assert!(!message.contains("Secret-Name"));
}
