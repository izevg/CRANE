#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use crate::config::Settings;
use crate::config::network::PUBLIC_URL;
use crate::config::storage::{BLOBS, DATABASE_URL};
use crate::config::tests::{MIN, env, error_of, file_lookup, no_files, with};
use crate::config::token::{TOKEN, TOKEN_FILE};

#[test]
fn token_comes_from_file_without_trailing_newline() {
    // Arrange
    let pairs: Vec<_> = with(&[(TOKEN_FILE, "/run/secrets/token")])
        .into_iter()
        .filter(|(k, _)| *k != TOKEN)
        .collect();
    let read = |path: &Path| {
        (path == Path::new("/run/secrets/token"))
            .then(|| "file-token\r\n".to_owned())
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    };

    // Act
    let settings = Settings::from_env(&env(&pairs), &read).unwrap();

    // Assert
    assert_eq!(settings.token.expose(), "file-token");
}

#[test]
fn token_file_keeps_inner_whitespace() {
    // Arrange
    let pairs: Vec<_> = with(&[(TOKEN_FILE, "/t")])
        .into_iter()
        .filter(|(k, _)| *k != TOKEN)
        .collect();
    let read = |_: &Path| Ok(" a b \n".to_owned());

    // Act
    let settings = Settings::from_env(&env(&pairs), &read).unwrap();

    // Assert
    assert_eq!(settings.token.expose(), " a b ");
}

#[test]
fn token_and_token_file_together_conflict() {
    // Arrange
    let pairs = with(&[(TOKEN_FILE, "/t")]);

    // Act
    let error = error_of(&pairs);

    // Assert
    assert_eq!(
        error,
        ConfigError::Conflict {
            first: TOKEN,
            second: TOKEN_FILE
        }
    );
}

#[test]
fn missing_token_is_reported() {
    // Arrange
    let pairs: Vec<_> = MIN.iter().filter(|(k, _)| *k != TOKEN).copied().collect();

    // Act
    let error = error_of(&pairs);

    // Assert
    assert_eq!(error, ConfigError::Missing { var: TOKEN });
}

#[test]
fn empty_tokens_are_rejected_from_env_and_from_file() {
    // Arrange
    let from_env = with(&[(TOKEN, "")]);
    let from_file: Vec<_> = with(&[(TOKEN_FILE, "/t")])
        .into_iter()
        .filter(|(k, _)| *k != TOKEN)
        .collect();
    let read = |_: &Path| Ok("\n".to_owned());

    // Act
    let errors =
        [from_env, from_file].map(|pairs| Settings::from_env(&env(&pairs), &read).err().unwrap());

    // Assert
    let empty = || ConfigError::Invalid {
        var: TOKEN,
        reason: "значение пусто",
    };
    assert_eq!(errors, [empty(), empty()]);
}

#[test]
fn unreadable_token_file_is_reported_without_the_token() {
    // Arrange
    let pairs: Vec<_> = with(&[(TOKEN_FILE, "/missing")])
        .into_iter()
        .filter(|(k, _)| *k != TOKEN)
        .collect();

    // Act
    let error = error_of(&pairs);

    // Assert
    assert!(matches!(
        error,
        ConfigError::SecretFile {
            var: TOKEN_FILE,
            ..
        }
    ));
}

#[test]
fn token_in_file_conflicts_with_token_file_in_env() {
    // Arrange
    let get = file_lookup(
        &[
            (DATABASE_URL, "memory"),
            (BLOBS, "memory"),
            (PUBLIC_URL, "https://x.example"),
            (TOKEN_FILE, "/t"),
        ],
        "token = \"file-token\"\n",
    );

    // Act
    let error = Settings::from_env(&get, &no_files).err().unwrap();

    // Assert
    assert_eq!(
        error,
        ConfigError::Conflict {
            first: TOKEN,
            second: TOKEN_FILE
        }
    );
}
