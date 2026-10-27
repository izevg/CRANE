#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use crate::config::tests::{env, error_of, file_lookup, no_files, parse, with};

#[test]
fn proxy_artifact_limit_defaults_and_accepts_explicit_value() {
    // Arrange
    let cases = [with(&[]), with(&[(PROXY_MAX_ARTIFACT_BYTES, "4096")])];

    // Act
    let actual = cases.map(|pairs| parse(&pairs).unwrap().proxy.max_artifact_bytes);

    // Assert
    assert_eq!(actual, [2_147_483_648, 4096]);
}

#[test]
fn invalid_proxy_artifact_limit_is_rejected_naming_the_variable() {
    // Arrange
    let cases = [
        with(&[(PROXY_MAX_ARTIFACT_BYTES, "0")]),
        with(&[(PROXY_MAX_ARTIFACT_BYTES, "abc")]),
    ];

    // Act
    let actual = cases.map(|pairs| error_of(&pairs));

    // Assert
    assert_eq!(
        actual,
        [
            ConfigError::Invalid {
                var: PROXY_MAX_ARTIFACT_BYTES,
                reason: "предел должен быть больше нуля"
            },
            ConfigError::Invalid {
                var: PROXY_MAX_ARTIFACT_BYTES,
                reason: "ожидалось целое число в допустимом диапазоне"
            }
        ]
    );
}

fn cache_kind(result: Result<Cache, ConfigError>) -> Result<&'static str, ConfigError> {
    result.map(|cache| match cache {
        Cache::Memory => "memory",
        Cache::Valkey(_) => "valkey",
    })
}

#[test]
fn cache_is_optional_and_selected_by_value() {
    // Arrange
    let cases: [&[(&str, &str)]; 5] = [
        &[],
        &[(CACHE, "memory")],
        &[(CACHE, "valkey://h:6379")],
        &[(CACHE, "valkeys://u:secret@h:6380/1")],
        &[(CACHE, "valkey-cluster://h:6379")],
    ];

    // Act
    let actual = cases.map(|pairs| cache_kind(Cache::from_env(&env(pairs), &no_files)));

    // Assert
    assert_eq!(
        actual,
        [
            Ok("memory"),
            Ok("memory"),
            Ok("valkey"),
            Ok("valkey"),
            Ok("valkey")
        ]
    );
}

#[test]
fn cache_rejects_foreign_scheme_and_malformed_address() {
    // Arrange
    let cases = ["redis://u:secret@h", "p@ss word://x", "valkey://[::1"];

    // Act
    let actual =
        cases.map(|value| cache_kind(Cache::from_env(&env(&[(CACHE, value)]), &no_files)).err());

    // Assert
    let invalid = || {
        Some(ConfigError::Invalid {
            var: CACHE,
            reason: "ожидалось memory или valkey[s][-cluster|-sentinel]://…",
        })
    };
    assert_eq!(
        actual,
        [
            Some(ConfigError::Unsupported {
                var: CACHE,
                scheme: "redis".to_owned()
            }),
            invalid(),
            invalid(),
        ]
    );
}

#[test]
fn cache_errors_never_quote_the_address() {
    // Arrange
    let cases = [
        "redis://u:secret@h",
        "user:secret@h://x",
        "valkey://u:secret@[::1",
    ];

    // Act
    let messages = cases.map(|value| {
        Cache::from_env(&env(&[(CACHE, value)]), &no_files)
            .err()
            .unwrap()
            .to_string()
    });

    // Assert
    assert!(messages.iter().all(|message| !message.contains("secret")));
}

#[test]
fn cache_url_and_its_file_together_conflict() {
    // Arrange
    let source = env(&[(CACHE, "memory"), (CACHE_FILE, "/cache")]);

    // Act
    let error = Cache::from_env(&source, &no_files).err().unwrap();

    // Assert
    assert_eq!(
        error,
        ConfigError::Conflict {
            first: CACHE,
            second: CACHE_FILE
        }
    );
}

#[test]
fn cache_file_is_read_without_trailing_newline() {
    // Arrange
    let source = env(&[(CACHE_FILE, "/run/secrets/cache")]);
    let read = |path: &Path| {
        (path == Path::new("/run/secrets/cache"))
            .then(|| "valkey://:pw@h:6379\n".to_owned())
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    };

    // Act
    let cache = cache_kind(Cache::from_env(&source, &read));

    // Assert
    assert_eq!(cache, Ok("valkey"));
}

#[test]
fn cache_key_in_config_file_is_accepted() {
    // Arrange
    let get = file_lookup(&[], "cache = \"valkey://h:6379\"\n");

    // Act
    let cache = cache_kind(Cache::from_env(&get, &no_files));

    // Assert
    assert_eq!(cache, Ok("valkey"));
}
