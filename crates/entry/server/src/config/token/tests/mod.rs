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

fn bootstrap_from(
    pairs: &[(&'static str, &'static str)],
    file: Option<&'static str>,
) -> Result<Option<String>, ConfigError> {
    let var = crate::config::tests::env(pairs);
    let read = move |_: &Path| {
        file.map(str::to_owned)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    };
    bootstrap_token(&var, &read).map(|token| token.map(|t| t.expose().to_owned()))
}

const LONG: &str = "crb_0123456789012345678901234567890123456789";

#[test]
fn bootstrap_token_comes_from_the_variable_or_the_file_with_the_newline_trimmed() {
    // Arrange
    let from_var = [(BOOTSTRAP_TOKEN, LONG)];
    let from_file = [(BOOTSTRAP_TOKEN_FILE, "/run/secrets/bootstrap")];

    // Act
    let results = [
        bootstrap_from(&from_var, None),
        bootstrap_from(
            &from_file,
            Some("crb_0123456789012345678901234567890123456789\n"),
        ),
        bootstrap_from(&[], None),
    ];

    // Assert
    assert_eq!(
        results,
        [
            Ok(Some(LONG.to_owned())),
            Ok(Some(LONG.to_owned())),
            Ok(None)
        ]
    );
}

#[test]
fn bootstrap_token_without_prefix_or_too_short_is_invalid() {
    // Arrange
    let cases = [
        [(BOOTSTRAP_TOKEN, "0123456789012345678901234567890123456789")],
        [(BOOTSTRAP_TOKEN, "crb_0123456789012345678901234567")],
    ];

    // Act
    let errors = cases.map(|pairs| bootstrap_from(&pairs, None));

    // Assert
    let invalid = || {
        Err(ConfigError::Invalid {
            var: BOOTSTRAP_TOKEN,
            reason: "ожидался токен вида crb_… не короче 36 символов",
        })
    };
    assert_eq!(errors, [invalid(), invalid()]);
}

#[test]
fn bootstrap_token_boundary_length_is_accepted() {
    // Arrange
    let at_limit = [(BOOTSTRAP_TOKEN, "crb_01234567890123456789012345678901")];

    // Act
    let result = bootstrap_from(&at_limit, None);

    // Assert
    assert_eq!(
        result,
        Ok(Some("crb_01234567890123456789012345678901".to_owned()))
    );
}

#[test]
fn static_token_is_set_by_variable_or_file_only() {
    // Arrange
    let cases = [
        vec![(TOKEN, "x")],
        vec![(TOKEN_FILE, "/run/secrets/token")],
        vec![(BOOTSTRAP_TOKEN, LONG)],
        vec![],
    ];

    // Act
    let set = cases.map(|pairs| static_token_set(&crate::config::tests::env(&pairs)));

    // Assert
    assert_eq!(set, [true, true, false, false]);
}
