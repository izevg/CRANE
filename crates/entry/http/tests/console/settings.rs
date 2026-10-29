//! `GET/PUT/DELETE /settings`: представление, условия записи, отказы.

use serde_json::json;

use crate::fixture::{Console, ENV_KEY, ENV_SECRET_KEY};

fn names(expected: &[&str]) -> Vec<String> {
    expected.iter().map(|name| (*name).to_owned()).collect()
}

fn keys(reply: &crate::common::Reply) -> Vec<String> {
    reply.json()["_embedded"]["crane:settings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["key"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn list_shows_every_console_key_with_the_write_mode_and_no_pending_restart() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get("/api/v1/settings");

    // Assert
    let body = reply.json();
    assert_eq!(
        (keys(&reply), &body["persistent"], &body["restart_required"]),
        (
            names(&[
                "blobs",
                "max_publish_bytes",
                "s3_secret",
                "registry_token",
                ENV_KEY,
                ENV_SECRET_KEY
            ]),
            &json!(true),
            &json!(false)
        )
    );
}

#[test]
fn list_reports_that_nothing_can_be_written_in_memory_mode() {
    // Arrange
    let console = Console::read_only();

    // Act
    let reply = console.get("/api/v1/settings");

    // Assert
    assert_eq!(reply.json()["persistent"], false);
}

#[test]
fn list_filters_by_group_and_by_origin() {
    // Arrange
    let console = Console::new();
    let queries = [
        "?group=storage",
        "?origin=env",
        "?group=storage&origin=env",
        "?group=nothing",
    ];

    // Act
    let found = queries.map(|query| keys(&console.get(&format!("/api/v1/settings{query}"))));

    // Assert
    assert_eq!(
        found,
        [
            names(&["blobs", "s3_secret"]),
            names(&[ENV_KEY, ENV_SECRET_KEY]),
            names(&[]),
            names(&[])
        ]
    );
}

#[test]
fn list_rejects_an_unknown_origin() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get("/api/v1/settings?origin=bogus");

    // Assert
    assert_eq!(reply.status.as_u16(), 400);
}

#[test]
fn an_untouched_key_is_a_default_without_a_row() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get("/api/v1/settings/max_publish_bytes");

    // Assert
    let body = reply.json();
    assert_eq!(
        (
            &body["origin"],
            &body["locked"],
            &body["value"],
            &body["version"],
            &body["unit"],
            &body["kind"],
            &body["_links"]["self"]["href"],
        ),
        (
            &json!("default"),
            &json!(false),
            &json!(null),
            &json!(null),
            &json!("bytes"),
            &json!("number"),
            &json!("/api/v1/settings/max_publish_bytes"),
        )
    );
    assert!(reply.headers.get("etag").is_none());
}

#[test]
fn an_environment_key_is_locked_and_names_its_variable() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get(&format!("/api/v1/settings/{ENV_KEY}"));

    // Assert
    let body = reply.json();
    assert_eq!(
        (
            &body["origin"],
            &body["locked"],
            &body["value"],
            &body["env_var"]
        ),
        (
            &json!("env"),
            &json!(true),
            &json!("8080"),
            &json!("CRANE_HTTP_PORT")
        )
    );
}

#[test]
fn a_stored_row_hidden_by_the_environment_is_shown_with_its_date_and_author() {
    // Arrange
    let console = Console::with_shadowed_row();

    // Act
    let reply = console.get(&format!("/api/v1/settings/{ENV_KEY}"));

    // Assert
    let body = reply.json();
    assert_eq!(
        (&body["origin"], &body["value"], &body["shadowed"]),
        (
            &json!("env"),
            &json!("8080"),
            &json!({"version": 1, "updated_at": "2023-11-14T22:13:20Z", "updated_by": "admin"})
        )
    );
}

#[test]
fn a_key_without_a_hidden_row_has_no_shadowed_part() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get(&format!("/api/v1/settings/{ENV_KEY}"));

    // Assert
    assert_eq!(reply.json()["shadowed"], json!(null));
}
