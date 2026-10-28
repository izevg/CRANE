#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;
use std::collections::HashMap;

use crane_domain::registry::{Format, RepositoryKind};

use super::network::{ADMIN_PORT, HTTP_PORT, PUBLIC_URL};
use super::repos::MAX_PUBLISH_BYTES;
use super::storage::{BLOBS, DATABASE_URL};
use super::token::TOKEN;

pub(in crate::config) fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |key| map.get(key).cloned()
}

pub(in crate::config) fn no_files(_: &Path) -> io::Result<String> {
    Err(io::Error::new(io::ErrorKind::NotFound, "нет файла"))
}

pub(in crate::config) const MIN: [(&str, &str); 4] = [
    (DATABASE_URL, "memory"),
    (BLOBS, "memory"),
    (PUBLIC_URL, "https://crane.example"),
    (TOKEN, "t"),
];

pub(in crate::config) fn with(
    extra: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    let mut all: Vec<_> = MIN
        .iter()
        .filter(|(k, _)| !extra.iter().any(|(ek, _)| ek == k))
        .copied()
        .collect();
    all.extend_from_slice(extra);
    all
}

pub(in crate::config) fn parse(pairs: &[(&str, &str)]) -> Result<Settings, ConfigError> {
    Settings::from_env(&env(pairs), &no_files)
}

pub(in crate::config) fn error_of(pairs: &[(&str, &str)]) -> ConfigError {
    parse(pairs).err().unwrap()
}

pub(in crate::config) fn file_lookup(
    env: &[(&str, &str)],
    content: &'static str,
) -> impl Fn(&str) -> Option<String> + use<> {
    let mut all = vec![("CRANE_CONFIG", "/etc/crane.toml")];
    all.extend_from_slice(env);
    layered(self::env(&all), &move |_: &Path| Ok(content.to_owned()))
        .ok()
        .unwrap()
}

#[test]
fn minimal_env_gets_defaults() {
    // Arrange
    let pairs = with(&[]);

    // Act
    let settings = parse(&pairs).unwrap();

    // Assert
    assert_eq!(settings.http_port, 8080);
    assert_eq!(settings.admin_port, 9464);
    assert_eq!(
        settings.repos.names(Format::Cargo, RepositoryKind::Hosted),
        ["internal"]
    );
    assert_eq!(settings.max_publish_bytes, 10 * 1024 * 1024);
    assert_eq!(settings.public_url, "https://crane.example");
    assert_eq!(settings.token.expose(), "t");
    assert!(matches!(settings.storage, Storage::Memory));
}

#[test]
fn explicit_values_override_defaults() {
    // Arrange
    let pairs = with(&[
        (HTTP_PORT, "1"),
        (ADMIN_PORT, "65535"),
        (MAX_PUBLISH_BYTES, "123"),
    ]);

    // Act
    let settings = parse(&pairs).unwrap();

    // Assert
    assert_eq!(
        (
            settings.http_port,
            settings.admin_port,
            settings.max_publish_bytes
        ),
        (1, 65535, 123)
    );
}

#[test]
fn required_variables_report_which_one_is_missing() {
    // Arrange
    let cases = [DATABASE_URL, BLOBS, PUBLIC_URL];

    // Act
    let errors = cases.map(|missing| {
        let pairs: Vec<_> = MIN.iter().filter(|(k, _)| *k != missing).copied().collect();
        error_of(&pairs)
    });

    // Assert
    assert_eq!(
        errors,
        [DATABASE_URL, BLOBS, PUBLIC_URL].map(|var| ConfigError::Missing { var })
    );
}

#[test]
fn storage_from_env_does_not_need_token_or_public_url() {
    // Arrange
    let source = env(&[(DATABASE_URL, "memory"), (BLOBS, "memory")]);

    // Act
    let storage = Storage::from_env(&source, &no_files).unwrap();

    // Assert
    assert!(matches!(storage, Storage::Memory));
}
