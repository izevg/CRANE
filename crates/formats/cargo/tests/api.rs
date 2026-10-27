#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::registry::{Format, PackageName, Version};
use crane_format_cargo::{asset_path, render_config, render_errors};
use serde_json::{Value, json};

fn parsed(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap()
}

#[test]
fn asset_path_uses_normalized_name_and_canonical_version() {
    // Arrange
    let name = PackageName::parse(Format::Cargo, "Foo-Bar").unwrap();
    let version = Version::parse(Format::Cargo, "1.2.3-rc.1+build5").unwrap();

    // Act
    let path = asset_path(&name, &version);

    // Assert
    assert_eq!(path, "foo_bar-1.2.3-rc.1.crate");
}

#[test]
fn config_points_dl_to_crates_api_and_requires_auth() {
    // Arrange
    let base = "https://reg.example/r/cargo";

    // Act
    let json = render_config(base);

    // Assert
    assert_eq!(
        parsed(&json),
        json!({
            "dl": "https://reg.example/r/cargo/api/v1/crates",
            "api": "https://reg.example/r/cargo",
            "auth-required": true,
        })
    );
}

#[test]
fn errors_wrap_detail_and_escape_it() {
    // Arrange
    let detail = "плохой \"ввод\"";

    // Act
    let json = render_errors(detail);

    // Assert
    assert_eq!(
        parsed(&json),
        json!({"errors": [{"detail": "плохой \"ввод\""}]})
    );
}
