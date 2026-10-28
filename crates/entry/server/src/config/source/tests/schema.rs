#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::SystemTime;

use crane_domain::settings::{Secret, StoredValue, Unreadable, Version};

fn schema(pairs: &[(&str, &str)], persistent: bool) -> ConfigSchema {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    ConfigSchema::new(
        Arc::new(move |name: &str| map.get(name).cloned()),
        Arc::new(|_: &Path| Err(io::Error::from(io::ErrorKind::NotFound))),
        persistent,
    )
}

fn rows(items: Vec<(&str, StoredValue)>) -> BTreeMap<String, Stored> {
    items
        .into_iter()
        .map(|(key, value)| {
            let row = Stored {
                value,
                version: Version::FIRST,
                updated_at: SystemTime::UNIX_EPOCH,
                updated_by: "admin".to_owned(),
            };
            (key.to_owned(), row)
        })
        .collect()
}

fn plain(text: &str) -> NewValue {
    NewValue::Plain(text.to_owned())
}

/// Ключи в ошибках проверки: значения и тексты ошибок тесту не нужны.
fn failed_keys(result: Result<(), Invalid>) -> Option<Vec<String>> {
    result
        .err()
        .map(|invalid| invalid.errors.into_iter().map(|error| error.key).collect())
}

#[test]
fn inherited_reports_variable_and_value_but_never_a_secret() {
    // Arrange
    let schema = schema(
        &[
            ("CRANE_TOKEN_PRINCIPAL", "robot"),
            ("CRANE_TOKEN_FILE", "/run/secrets/token"),
            ("CRANE_CACHE", "valkey://u:SECRET@h"),
        ],
        false,
    );

    // Act
    let actual = [
        "token_principal",
        "token",
        "cache",
        "namespaces",
        "no_such_key",
    ]
    .map(|key| schema.inherited(key));

    // Assert
    assert_eq!(
        actual,
        [
            Some(Inherited {
                var: "CRANE_TOKEN_PRINCIPAL".to_owned(),
                value: Some("robot".to_owned())
            }),
            Some(Inherited {
                var: "CRANE_TOKEN_FILE".to_owned(),
                value: None
            }),
            Some(Inherited {
                var: "CRANE_CACHE".to_owned(),
                value: None
            }),
            None,
            None,
        ]
    );
}

#[test]
fn check_accepts_valid_candidate_and_rejects_invalid_one_naming_its_key() {
    // Arrange
    let schema = schema(&[], false);
    let good = plain("https://crane.example");
    let bad = plain("ftp://crane.example");

    // Act
    let results = [&good, &bad].map(|value| {
        failed_keys(schema.check("network", &rows(vec![]), &[("public_url", Some(value))]))
    });

    // Assert
    assert_eq!(results, [None, Some(vec!["public_url".to_owned()])]);
}

#[test]
fn check_of_a_section_ignores_failures_of_other_sections() {
    // Arrange
    // Токен ещё не задан, но проверяется только секция network.
    let schema = schema(&[], false);
    let value = plain("https://crane.example");

    // Act
    let result = schema.check("network", &rows(vec![]), &[("public_url", Some(&value))]);

    // Assert
    assert_eq!(failed_keys(result), None);
}

#[test]
fn deleting_a_required_key_fails_the_check_of_its_key() {
    // Arrange
    let schema = schema(&[], true);
    let stored = rows(vec![(
        "blobs",
        StoredValue::Plain("file:///data".to_owned()),
    )]);

    // Act
    let result = schema.check("storage", &stored, &[("blobs", None)]);

    // Assert
    assert_eq!(failed_keys(result), Some(vec!["blobs".to_owned()]));
}

#[test]
fn blobs_candidate_is_checked_against_the_catalog_mode() {
    // Arrange
    let durable = schema(&[], true);
    let memory = schema(&[], false);
    let in_memory = plain("memory");
    let on_disk = plain("file:///data");

    // Act
    let results = [
        failed_keys(durable.check("storage", &rows(vec![]), &[("blobs", Some(&in_memory))])),
        failed_keys(durable.check("storage", &rows(vec![]), &[("blobs", Some(&on_disk))])),
        failed_keys(memory.check("storage", &rows(vec![]), &[("blobs", Some(&on_disk))])),
    ];

    // Assert
    assert_eq!(
        results,
        [
            Some(vec!["blobs".to_owned()]),
            None,
            Some(vec!["blobs".to_owned()])
        ]
    );
}

#[test]
fn unreadable_secret_inside_the_section_fails_its_own_key() {
    // Arrange
    let schema = schema(&[], false);
    let stored = rows(vec![(
        "token",
        StoredValue::Unreadable(Unreadable::Corrupted),
    )]);

    // Act
    let result = schema.check("token", &stored, &[]);

    // Assert
    assert_eq!(failed_keys(result), Some(vec!["token".to_owned()]));
}

#[test]
fn unreadable_secret_of_another_section_does_not_fail_this_one() {
    // Arrange
    let schema = schema(&[], false);
    let stored = rows(vec![(
        "token",
        StoredValue::Unreadable(Unreadable::Corrupted),
    )]);
    let value = plain("https://crane.example");

    // Act
    let result = schema.check("network", &stored, &[("public_url", Some(&value))]);

    // Assert
    assert_eq!(failed_keys(result), None);
}

#[test]
fn unreadable_secret_shadowed_by_the_environment_does_not_fail_the_section() {
    // Arrange
    let schema = schema(&[("CRANE_TOKEN", "from-env")], false);
    let stored = rows(vec![(
        "token",
        StoredValue::Unreadable(Unreadable::Corrupted),
    )]);

    // Act
    let result = schema.check("token", &stored, &[]);

    // Assert
    assert_eq!(failed_keys(result), None);
}

#[test]
fn secret_file_failure_is_reported_under_the_secret_key() {
    // Arrange
    let schema = schema(&[("CRANE_TOKEN_FILE", "/missing")], false);

    // Act
    let result = schema.check("token", &rows(vec![]), &[]);

    // Assert
    assert_eq!(failed_keys(result), Some(vec!["token".to_owned()]));
}

#[test]
fn check_detail_never_contains_secret_values() {
    // Arrange
    let schema = schema(&[], false);
    let secret = NewValue::Secret(Secret::new("SECRETvalue".to_owned()));
    let bad_cache = NewValue::Secret(Secret::new("ftp://u:SECRETpw@h".to_owned()));

    // Act
    let result = schema.check(
        "proxy",
        &rows(vec![]),
        &[("cache", Some(&bad_cache)), ("token", Some(&secret))],
    );

    // Assert
    let invalid = result.err().unwrap();
    let leaked: Vec<bool> = invalid
        .errors
        .iter()
        .map(|error| error.detail.contains("SECRET"))
        .collect();
    assert_eq!(leaked, [false]);
}
