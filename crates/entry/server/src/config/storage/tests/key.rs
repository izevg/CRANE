#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use crate::config::storage::{SETTINGS_KEY, SETTINGS_KEY_FILE};
use crate::config::tests::{env, no_files};

#[test]
fn settings_key_accepts_base64_of_32_bytes_and_ignores_absence() {
    // Arrange
    let valid = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
    let cases = [(env(&[]), true), (env(&[(SETTINGS_KEY, valid)]), true)];

    // Act
    let results = cases.map(|(source, persistent)| {
        settings_key(&source, &no_files, persistent).map(|key| key.map(|key| key.id().len()))
    });

    // Assert
    assert_eq!(results, [Ok(None), Ok(Some(16))]);
}

#[test]
fn settings_key_is_read_from_file_without_trailing_newline() {
    // Arrange
    let source = env(&[(SETTINGS_KEY_FILE, "/k")]);
    let read = |_: &Path| Ok("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=\n".to_owned());

    // Act
    let key = settings_key(&source, &read, true);

    // Assert
    assert!(key.is_ok_and(|key| key.is_some()));
}

#[test]
fn settings_key_errors_name_the_problem_but_never_the_content() {
    // Arrange
    let cases = ["SECRETnot base64!", "U0VDUkVU"];

    // Act
    let messages = cases.map(|value| {
        settings_key(&env(&[(SETTINGS_KEY, value)]), &no_files, true)
            .err()
            .map(|error| error.to_string())
    });

    // Assert
    let leaked = messages.map(|message| message.is_none_or(|text| text.contains("SECRET")));
    assert_eq!(leaked, [false; 2]);
}

#[test]
fn settings_key_is_rejected_with_memory_database() {
    // Arrange
    let source = env(&[(SETTINGS_KEY, "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=")]);
    let from_file = env(&[(SETTINGS_KEY_FILE, "/k")]);

    // Act
    let errors = [
        settings_key(&source, &no_files, false).err(),
        settings_key(&from_file, &no_files, false).err(),
    ];

    // Assert
    assert_eq!(
        errors,
        [
            Some(ConfigError::SettingsKeyWithMemory),
            Some(ConfigError::SettingsKeyWithMemory)
        ]
    );
}

#[test]
fn settings_key_and_its_file_together_conflict() {
    // Arrange
    let source = env(&[(SETTINGS_KEY, "x"), (SETTINGS_KEY_FILE, "/k")]);

    // Act
    let error = settings_key(&source, &no_files, true).err();

    // Assert
    assert_eq!(
        error,
        Some(ConfigError::Conflict {
            first: SETTINGS_KEY,
            second: SETTINGS_KEY_FILE
        })
    );
}
