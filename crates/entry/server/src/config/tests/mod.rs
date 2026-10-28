#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::SystemTime;

use crane_domain::registry::{Format, RepositoryKind};
use crane_domain::settings::{Kind, Stored, StoredValue, Unit, Version};

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

pub(in crate::config) fn parse(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
    Config::from_env(&env(pairs), &no_files, false)
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
    assert!(matches!(settings.blobs, Blobs::Memory));
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
    let cases = [BLOBS, PUBLIC_URL];

    // Act
    let errors = cases.map(|missing| {
        let pairs: Vec<_> = MIN.iter().filter(|(k, _)| *k != missing).copied().collect();
        error_of(&pairs)
    });

    // Assert
    assert_eq!(
        errors,
        [BLOBS, PUBLIC_URL].map(|var| ConfigError::Missing { var })
    );
}

#[test]
fn database_from_env_does_not_need_token_blobs_or_public_url() {
    // Arrange
    let source = env(&[(DATABASE_URL, "memory")]);

    // Act
    let database = Database::from_env(&source, &no_files).unwrap();

    // Assert
    assert!(!database.persistent());
}

fn plain_rows(items: &[(&str, &str)]) -> BTreeMap<String, Stored> {
    items
        .iter()
        .map(|(key, value)| {
            let row = Stored {
                value: StoredValue::Plain((*value).to_owned()),
                version: Version::FIRST,
                updated_at: SystemTime::UNIX_EPOCH,
                updated_by: "admin".to_owned(),
            };
            ((*key).to_owned(), row)
        })
        .collect()
}

fn layered_config(
    inherited: &[(&str, &str)],
    rows: &[(&str, &str)],
) -> Result<Config, ConfigError> {
    let inherited = env(inherited);
    let rows = plain_rows(rows);
    let (layer, _) = StoredLayer::new(&inherited, &rows, &[])?;
    Config::from_layers(&inherited, &layer, &no_files, false)
}

#[test]
fn stored_value_acts_when_the_environment_is_silent() {
    // Arrange
    let inherited = with(&[]);

    // Act
    let config = layered_config(&inherited, &[("token_principal", "from-db")]);

    // Assert
    assert_eq!(config.unwrap().principal.id, "from-db");
}

#[test]
fn environment_overrides_the_stored_value() {
    // Arrange
    let inherited = with(&[("CRANE_TOKEN_PRINCIPAL", "from-env")]);

    // Act
    let config = layered_config(&inherited, &[("token_principal", "from-db")]);

    // Assert
    assert_eq!(config.unwrap().principal.id, "from-env");
}

#[test]
fn stored_value_fills_a_required_key_the_environment_lacks() {
    // Arrange
    let inherited: Vec<_> = with(&[])
        .into_iter()
        .filter(|(k, _)| *k != PUBLIC_URL)
        .collect();

    // Act
    let config = layered_config(&inherited, &[("public_url", "https://db.example")]);

    // Assert
    assert_eq!(config.unwrap().public_url, "https://db.example");
}

#[test]
fn token_file_in_environment_shadows_stored_token_instead_of_conflicting() {
    // Arrange
    let inherited: Vec<_> = with(&[("CRANE_TOKEN_FILE", "/run/secrets/token")])
        .into_iter()
        .filter(|(k, _)| *k != TOKEN)
        .collect();
    let inherited = env(&inherited);
    let rows = plain_rows(&[("token", "stored-token")]);
    let read = |_: &Path| Ok("file-token\n".to_owned());
    let (layer, _) = StoredLayer::new(&inherited, &rows, &[]).unwrap();

    // Act
    let config = Config::from_layers(&inherited, &layer, &read, false);

    // Assert
    assert_eq!(config.unwrap().token.expose(), "file-token");
}

#[test]
fn stored_row_of_a_bootstrap_only_key_does_not_change_it() {
    // Arrange
    let inherited = with(&[]);

    // Act
    let config = layered_config(
        &inherited,
        &[("http_port", "1"), ("trusted_proxies", "10.0.0.0/8")],
    );

    // Assert
    assert_eq!(config.unwrap().http_port, 8080);
}

#[test]
fn every_definition_key_is_its_variable_without_prefix_in_lowercase() {
    // Arrange
    let defs = definitions();

    // Act
    let mismatched: Vec<_> = defs
        .iter()
        .filter(|def| {
            def.env_var
                .strip_prefix("CRANE_")
                .map(str::to_ascii_lowercase)
                .as_deref()
                != Some(def.key)
        })
        .map(|def| def.key)
        .collect();

    // Assert
    assert_eq!(mismatched, Vec::<&str>::new());
}

#[test]
fn variable_lists_match_the_definitions_plus_file_variables_of_secrets() {
    // Arrange
    let from_definitions: BTreeSet<String> = definitions()
        .iter()
        .flat_map(|def| {
            let file = (def.kind == Kind::Secret).then(|| format!("{}_FILE", def.env_var));
            [Some(def.env_var.to_owned()), file]
        })
        .flatten()
        .collect();

    // Act
    let from_vars: BTreeSet<String> = vars().map(str::to_owned).collect();

    // Assert
    assert_eq!(from_vars, from_definitions);
}

#[test]
fn bootstrap_keys_are_hidden_from_the_console() {
    // Arrange
    let keys = [
        "database_url",
        "http_port",
        "admin_port",
        "trusted_proxies",
        "settings_key",
    ];

    // Act
    let hidden = keys.map(|key| definition(key).map(|def| def.console));

    // Assert
    assert_eq!(hidden, [Some(false); 5]);
}

#[test]
fn secret_keys_have_the_secret_kind() {
    // Arrange
    let keys = ["token", "database_url", "cache"];

    // Act
    let kinds = keys.map(|key| definition(key).map(|def| def.kind));

    // Assert
    assert_eq!(kinds, [Some(Kind::Secret); 3]);
}

#[test]
fn repo_lists_are_defined_from_repo_vars_as_lists() {
    // Arrange
    let keys = [
        "cargo_repos",
        "cargo_proxy_repos",
        "npm_repos",
        "npm_proxy_repos",
    ];

    // Act
    let kinds = keys.map(|key| definition(key).map(|def| (def.kind, def.group)));

    // Assert
    assert_eq!(kinds, [Some((Kind::List, "repos")); 4]);
}

#[test]
fn error_variable_points_at_the_key_the_error_is_about() {
    // Arrange
    let errors = [
        ConfigError::Missing { var: BLOBS },
        ConfigError::Conflict {
            first: TOKEN,
            second: "CRANE_TOKEN_FILE",
        },
        ConfigError::MixedStorage,
        ConfigError::S3KeyWithMemory,
        ConfigError::SettingsKeyWithMemory,
        ConfigError::UnknownKey {
            key: "x".to_owned(),
        },
    ];

    // Act
    let vars = errors.map(|error| error.var());

    // Assert
    assert_eq!(
        vars,
        [
            Some(BLOBS),
            Some(TOKEN),
            Some(BLOBS),
            Some(BLOBS),
            Some("CRANE_SETTINGS_KEY"),
            None
        ]
    );
}

#[test]
fn byte_limits_carry_the_bytes_unit() {
    // Arrange
    let keys = ["max_publish_bytes", "proxy_max_artifact_bytes", "http_port"];

    // Act
    let units = keys.map(|key| definition(key).map(|def| def.unit));

    // Assert
    assert_eq!(
        units,
        [Some(Some(Unit::Bytes)), Some(Some(Unit::Bytes)), Some(None)]
    );
}
