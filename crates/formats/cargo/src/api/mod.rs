//! Ответы web API cargo и имя файла `.crate`.

use crane_domain::registry::{PackageName, Version};
use serde_json::json;

/// Путь файла `.crate` среди assets версии.
#[must_use]
pub fn asset_path(name: &PackageName, version: &Version) -> String {
    format!("{}-{}.crate", name.normalized(), version.canonical())
}

/// `repo_base` без завершающего '/'. Cargo сам дописывает `/{crate}/{version}/download`,
/// и это совпадает с маршрутом API.
#[must_use]
pub fn render_config(repo_base: &str) -> String {
    json!({
        "dl": format!("{repo_base}/api/v1/crates"),
        "api": repo_base,
        "auth-required": true,
    })
    .to_string()
}

#[must_use]
pub fn render_errors(detail: &str) -> String {
    json!({ "errors": [{ "detail": detail }] }).to_string()
}

#[must_use]
pub fn render_ok() -> String {
    json!({ "ok": true }).to_string()
}

#[must_use]
pub fn render_publish_ok() -> String {
    json!({ "warnings": { "invalid_categories": [], "invalid_badges": [], "other": [] } })
        .to_string()
}

/// Единственный владелец — вызывающий; реальных владельцев в модели доступа нет.
#[must_use]
pub fn render_owners(login: &str) -> String {
    json!({ "users": [{ "id": 1, "login": login, "name": null }] }).to_string()
}
