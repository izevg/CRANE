//! Deprecate и запросы, которые формат не поддерживает.

use crane_format_npm::{Write, WriteError, classify};
use serde_json::{Value, json};

use crate::common::{pkg, stored, version};

fn deprecate_body(versions: &Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "name": "left-pad",
        "dist-tags": {"latest": "1.1.0", "evil": "1.0.0"},
        "versions": versions,
    }))
    .unwrap()
}

#[test]
fn classify_deprecate_emits_only_real_changes_to_known_versions() {
    // Arrange
    let current = [
        stored("left-pad", "1.0.0", 1, 10, None, "{}"),
        stored("left-pad", "1.1.0", 2, 20, Some("old"), "{}"),
    ];
    let cases = [
        // Поля нет: без изменений (в том числе для версии, отсутствующей в теле).
        deprecate_body(&json!({"1.0.0": {"name": "left-pad"}})),
        // Поля нет у уже устаревшей версии: устаревание не снимается.
        deprecate_body(&json!({"1.1.0": {"name": "left-pad"}})),
        // Пустая строка у неустаревшей: без изменений; у устаревшей: снятие.
        deprecate_body(&json!({"1.0.0": {"deprecated": ""}})),
        deprecate_body(&json!({"1.1.0": {"deprecated": ""}})),
        deprecate_body(&json!({"1.0.0": {"deprecated": "use x"}})),
        // То же сообщение: без изменений.
        deprecate_body(&json!({"1.1.0": {"deprecated": "old"}})),
        // Неизвестная версия и ключ-не-версия игнорируются; пропавшая из тела 1.1.0 не трогается.
        deprecate_body(&json!({"9.9.9": {"deprecated": "x"}, "latest": {"deprecated": "x"}})),
        deprecate_body(&json!({})),
    ];

    // Act
    let actual = cases.map(|request| classify(&pkg("left-pad"), &request, &current));

    // Assert
    assert_eq!(
        actual,
        [
            Ok(Write::Deprecate(vec![])),
            Ok(Write::Deprecate(vec![])),
            Ok(Write::Deprecate(vec![])),
            Ok(Write::Deprecate(vec![(version("1.1.0"), None)])),
            Ok(Write::Deprecate(vec![(
                version("1.0.0"),
                Some("use x".to_owned())
            )])),
            Ok(Write::Deprecate(vec![])),
            Ok(Write::Deprecate(vec![])),
            Ok(Write::Deprecate(vec![])),
        ]
    );
}

#[test]
fn classify_deprecate_refuses_non_string_message() {
    // Arrange
    let current = [stored("left-pad", "1.0.0", 1, 10, None, "{}")];
    let request = deprecate_body(&json!({"1.0.0": {"deprecated": true}}));

    // Act
    let actual = classify(&pkg("left-pad"), &request, &current);

    // Assert
    assert_eq!(actual, Err(WriteError::DeprecatedNotString));
}

#[test]
fn classify_without_attachments_or_versions_is_unsupported() {
    // Arrange
    let request = br#"{"name":"left-pad","maintainers":[{"name":"x"}]}"#;

    // Act
    let actual = classify(&pkg("left-pad"), request, &[]);

    // Assert
    assert_eq!(actual, Ok(Write::Unsupported));
}
