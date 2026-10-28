#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use crate::config::tests::{parse as parse_settings, with};

const OWNER: &str = "0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b";

fn pattern(format: Format, raw: &str) -> NamespacePattern {
    NamespacePattern::parse(format, raw).unwrap()
}

#[test]
fn empty_value_means_no_namespaces() {
    // Arrange
    let cases = ["", "  "];

    // Act
    let actual = cases.map(|raw| parse(raw).unwrap());

    // Assert
    assert_eq!(actual, [Vec::new(), Vec::new()]);
}

#[test]
fn entry_without_owner_belongs_to_default_workspace() {
    // Arrange
    let raw = " cargo:acme-* ";

    // Act
    let actual = parse(raw).unwrap();

    // Assert
    assert_eq!(
        actual,
        [Namespace {
            format: Format::Cargo,
            pattern: pattern(Format::Cargo, "acme-*"),
            workspace: WorkspaceId::DEFAULT,
        }]
    );
}

#[test]
fn entry_with_uuid_belongs_to_that_workspace() {
    // Arrange
    let raw = format!("cargo:acme-*,cargo:acme-payments-*={OWNER}");

    // Act
    let actual = parse(&raw).unwrap();

    // Assert
    let owners: Vec<WorkspaceId> = actual.iter().map(|entry| entry.workspace).collect();
    assert_eq!(
        owners,
        [
            WorkspaceId::DEFAULT,
            WorkspaceId::new(Uuid::parse_str(OWNER).unwrap())
        ]
    );
}

#[test]
fn maven_exact_name_keeps_its_colon() {
    // Arrange
    let raw = "maven:com.acme:core";

    // Act
    let actual = parse(raw).unwrap();

    // Assert
    assert_eq!(actual[0].pattern, pattern(Format::Maven, "com.acme:core"));
}

#[test]
fn invalid_entries_are_rejected_with_reason() {
    // Arrange
    let cases = [
        "cargo:acme-*,,cargo:b-*",
        "acme-*",
        "gem:acme-*",
        "cargo:acme*",
        "cargo:acme-*=not-a-uuid",
        "cargo:acme-*,cargo:Acme_*",
    ];

    // Act
    let actual = cases.map(|raw| parse(raw).err().unwrap());

    // Assert
    assert_eq!(
        actual,
        [
            invalid("пустая запись в списке"),
            invalid("ожидается `формат:шаблон`"),
            invalid("неизвестный формат"),
            invalid("некорректный шаблон"),
            invalid("некорректный uuid workspace"),
            invalid("шаблон повторяется в списке"),
        ]
    );
}

#[test]
fn namespaces_variable_reaches_settings_and_bad_value_fails_config() {
    // Arrange
    let cases = [
        with(&[]),
        with(&[(NAMESPACES, "cargo:acme-*")]),
        with(&[(NAMESPACES, "cargo:acme-*=nope")]),
    ];

    // Act
    let actual =
        cases.map(|pairs| parse_settings(&pairs).map(|settings| settings.namespaces.len()));

    // Assert
    assert_eq!(
        actual,
        [
            Ok(0),
            Ok(1),
            Err(ConfigError::Invalid {
                var: NAMESPACES,
                reason: "некорректный uuid workspace",
            }),
        ]
    );
}
