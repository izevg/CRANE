#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use std::time::SystemTime;

use crane_domain::settings::{Secret, Version};

use crate::config::tests::env;

pub(in crate::config) fn row(value: StoredValue) -> Stored {
    Stored {
        value,
        version: Version::FIRST,
        updated_at: SystemTime::UNIX_EPOCH,
        updated_by: "admin".to_owned(),
    }
}

fn plain(text: &str) -> StoredValue {
    StoredValue::Plain(text.to_owned())
}

fn secret(text: &str) -> StoredValue {
    StoredValue::Secret(Secret::new(text.to_owned()))
}

fn wrong_key() -> StoredValue {
    StoredValue::Unreadable(Unreadable::WrongKey {
        stored_key_id: "aaaa1111".to_owned(),
        current_key_id: "bbbb2222".to_owned(),
    })
}

/// Что видит `Config` по переменной `var` и что отброшено; значения секретов в сводке нужны
/// только тесту, чтобы убедиться, что слой отдал ровно записанное.
fn outcome(key: &str, value: StoredValue, inherited: &[(&str, &str)], var: &str) -> String {
    let rows = BTreeMap::from([(key.to_owned(), row(value))]);
    match StoredLayer::new(&env(inherited), &rows, &[]) {
        Ok((layer, ignored)) => format!("{:?} {ignored:?}", layer.get(var)),
        Err(error) => format!("error: {error}"),
    }
}

#[test]
fn stored_row_acts_unless_the_environment_answers_for_the_key() {
    // Arrange
    let cases = [
        (
            "token_principal",
            plain("alice"),
            vec![],
            "CRANE_TOKEN_PRINCIPAL",
        ),
        ("token", secret("s3cret"), vec![], "CRANE_TOKEN"),
        (
            "token_principal",
            plain("alice"),
            vec![("CRANE_TOKEN_PRINCIPAL", "env")],
            "CRANE_TOKEN_PRINCIPAL",
        ),
        (
            "token",
            secret("s3cret"),
            vec![("CRANE_TOKEN", "env")],
            "CRANE_TOKEN",
        ),
        (
            "token",
            secret("s3cret"),
            vec![("CRANE_TOKEN_FILE", "/run/secrets/token")],
            "CRANE_TOKEN",
        ),
        // `_FILE` бывает только у секрета: у обычного ключа такая переменная ключа не затеняет.
        (
            "token_principal",
            plain("alice"),
            vec![("CRANE_TOKEN_PRINCIPAL_FILE", "/x")],
            "CRANE_TOKEN_PRINCIPAL",
        ),
    ];

    // Act
    let actual = cases.map(|(key, value, inherited, var)| outcome(key, value, &inherited, var));

    // Assert
    assert_eq!(
        actual,
        [
            r#"Some("alice") []"#.to_owned(),
            r#"Some("s3cret") []"#.to_owned(),
            r#"None [("token_principal", Shadowed { var: "CRANE_TOKEN_PRINCIPAL" })]"#.to_owned(),
            r#"None [("token", Shadowed { var: "CRANE_TOKEN" })]"#.to_owned(),
            r#"None [("token", Shadowed { var: "CRANE_TOKEN_FILE" })]"#.to_owned(),
            r#"Some("alice") []"#.to_owned(),
        ]
    );
}

#[test]
fn stored_rows_of_env_only_and_unknown_keys_are_dropped() {
    // Arrange
    let cases = [
        ("http_port", "CRANE_HTTP_PORT"),
        ("database_url", "CRANE_DATABASE_URL"),
        ("trusted_proxies", "CRANE_TRUSTED_PROXIES"),
        ("settings_key", "CRANE_SETTINGS_KEY"),
        ("no_such_key", "CRANE_NO_SUCH_KEY"),
    ];

    // Act
    let actual = cases.map(|(key, var)| outcome(key, plain("1"), &[], var));

    // Assert
    assert_eq!(
        actual,
        [
            r#"None [("http_port", EnvOnly)]"#,
            r#"None [("database_url", EnvOnly)]"#,
            r#"None [("trusted_proxies", EnvOnly)]"#,
            r#"None [("settings_key", EnvOnly)]"#,
            r#"None [("no_such_key", Unknown)]"#,
        ]
    );
}

#[test]
fn unreadable_secret_without_environment_fails_naming_key_and_both_fingerprints() {
    // Arrange
    let rows = BTreeMap::from([("token".to_owned(), row(wrong_key()))]);

    // Act
    let result = StoredLayer::new(&env(&[]), &rows, &[]);

    // Assert
    let message = result.err().unwrap().to_string();
    assert!(message.contains("`token`"), "{message}");
    assert!(message.contains("aaaa1111"), "{message}");
    assert!(message.contains("bbbb2222"), "{message}");
}

#[test]
fn unreadable_reasons_are_told_apart() {
    // Arrange
    let cases = [
        StoredValue::Unreadable(Unreadable::MissingKey),
        wrong_key(),
        StoredValue::Unreadable(Unreadable::Corrupted),
    ];

    // Act
    let messages = cases.map(|value| outcome("token", value, &[], "CRANE_TOKEN"));

    // Assert
    assert_eq!(
        messages,
        [
            "error: настройка `token` в БД не читается: задайте CRANE_SETTINGS_KEY: строка запечатана, а ключа нет",
            "error: настройка `token` в БД не читается: строка запечатана ключом aaaa1111, текущий ключ bbbb2222",
            "error: настройка `token` в БД не читается: шифртекст повреждён или принадлежит другой строке",
        ]
    );
}

#[test]
fn unreadable_secret_shadowed_by_the_environment_is_ignored_not_an_error() {
    // Arrange
    let inherited = [("CRANE_TOKEN_FILE", "/run/secrets/token")];

    // Act
    let actual = outcome("token", wrong_key(), &inherited, "CRANE_TOKEN");

    // Assert
    assert_eq!(
        actual,
        r#"None [("token", Shadowed { var: "CRANE_TOKEN_FILE" })]"#
    );
}

#[test]
fn changes_override_rows_and_deletion_removes_them() {
    // Arrange
    let rows = BTreeMap::from([
        ("token_principal".to_owned(), row(plain("old"))),
        ("namespaces".to_owned(), row(plain("cargo:acme-*"))),
        ("token".to_owned(), row(wrong_key())),
    ]);
    let new = NewValue::Plain("new".to_owned());
    let secret_value = NewValue::Secret(Secret::new("fresh".to_owned()));
    let changes = [
        ("token_principal", Some(&new)),
        ("namespaces", None),
        ("token", Some(&secret_value)),
    ];

    // Act
    let (layer, ignored) = StoredLayer::new(&env(&[]), &rows, &changes).unwrap();

    // Assert
    assert_eq!(
        (
            layer.get("CRANE_TOKEN_PRINCIPAL"),
            layer.get("CRANE_NAMESPACES"),
            layer.get("CRANE_TOKEN"),
            ignored
        ),
        (
            Some("new".to_owned()),
            None,
            Some("fresh".to_owned()),
            vec![]
        )
    );
}

#[test]
fn deleting_the_unreadable_row_clears_the_failure() {
    // Arrange
    let rows = BTreeMap::from([("token".to_owned(), row(wrong_key()))]);

    // Act
    let result = StoredLayer::new(&env(&[]), &rows, &[("token", None)]);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn inherited_var_names_the_variable_that_answered() {
    // Arrange
    let token = definition("token").unwrap();
    let principal = definition("token_principal").unwrap();
    let cases = [
        (token, vec![]),
        (token, vec![("CRANE_TOKEN", "x")]),
        (token, vec![("CRANE_TOKEN_FILE", "/f")]),
        (principal, vec![("CRANE_TOKEN_PRINCIPAL_FILE", "/f")]),
        (principal, vec![("CRANE_TOKEN_PRINCIPAL", "a")]),
    ];

    // Act
    let actual = cases.map(|(def, pairs)| inherited_var(def, &env(&pairs)));

    // Assert
    assert_eq!(
        actual,
        [
            None,
            Some("CRANE_TOKEN".to_owned()),
            Some("CRANE_TOKEN_FILE".to_owned()),
            None,
            Some("CRANE_TOKEN_PRINCIPAL".to_owned()),
        ]
    );
}
