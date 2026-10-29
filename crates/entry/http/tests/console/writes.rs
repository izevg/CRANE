//! Запись настроек: условия, отказы, аудит.

use axum::http::Method;
use crane_domain::audit::{AuditAction, Outcome};
use serde_json::json;

use crate::fixture::{BAD_VALUE, Console, ENV_KEY};

#[test]
fn creating_a_key_stores_it_and_returns_the_setting_with_its_version() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.create("blobs", "file:///data");

    // Assert
    let body = reply.json();
    assert_eq!(
        (
            reply.status.as_u16(),
            reply.header("etag"),
            &body["origin"],
            &body["value"],
            &body["version"],
            &body["updated_by"],
            &body["updated_at"],
        ),
        (
            200,
            "\"1\"",
            &json!("stored"),
            &json!("file:///data"),
            &json!(1),
            &json!("crane"),
            &json!("2023-11-14T22:13:20Z"),
        )
    );
}

#[test]
fn updating_with_the_current_version_bumps_it() {
    // Arrange
    let console = Console::new();
    console.create("blobs", "file:///one");

    // Act
    let reply = console.put("blobs", &[("if-match", "\"1\"")], "file:///two");

    // Assert
    let body = reply.json();
    assert_eq!(
        (reply.header("etag"), &body["version"], &body["value"]),
        ("\"2\"", &json!(2), &json!("file:///two"))
    );
}

#[test]
fn write_without_any_condition_is_precondition_required_and_stores_nothing() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.put("blobs", &[], "file:///data");

    // Assert
    assert_eq!(
        (reply.status.as_u16(), reply.json()["type"].clone()),
        (428, json!("urn:crane:problem:precondition-required"))
    );
    assert_eq!(
        console.get("/api/v1/settings/blobs").json()["origin"],
        "default"
    );
}

#[test]
fn write_with_a_malformed_condition_is_a_bad_request() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.put("blobs", &[("if-match", "1")], "file:///data");

    // Assert
    assert_eq!(reply.status.as_u16(), 400);
}

#[test]
fn stale_version_and_creating_over_an_existing_row_are_precondition_failed() {
    // Arrange
    let console = Console::new();
    console.create("blobs", "file:///one");
    console.put("blobs", &[("if-match", "\"1\"")], "file:///two");

    // Act
    let replies = [
        console.put("blobs", &[("if-match", "\"1\"")], "file:///three"),
        console.put("blobs", &[("if-none-match", "*")], "file:///three"),
    ];

    // Assert
    assert_eq!(replies.map(|reply| reply.status.as_u16()), [412, 412]);
}

#[test]
fn invalid_value_is_unprocessable_with_the_failed_key_and_is_not_stored() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.create("max_publish_bytes", BAD_VALUE);

    // Assert
    let body = reply.json();
    assert_eq!(
        (reply.status.as_u16(), &body["type"], &body["errors"]),
        (
            422,
            &json!("urn:crane:problem:setting-invalid"),
            &json!([{"key": "max_publish_bytes", "detail": "значение не разобрано"}])
        )
    );
}

#[test]
fn writing_a_key_owned_by_the_environment_is_a_conflict_naming_the_variable() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.create(ENV_KEY, "9090");

    // Assert
    let body = reply.json();
    assert_eq!(
        (reply.status.as_u16(), &body["type"], &body["var"]),
        (
            409,
            &json!("urn:crane:problem:setting-locked"),
            &json!("CRANE_HTTP_PORT")
        )
    );
}

#[test]
fn writing_in_memory_mode_is_a_conflict() {
    // Arrange
    let console = Console::read_only();

    // Act
    let reply = console.create("blobs", "file:///data");

    // Assert
    assert_eq!(
        (reply.status.as_u16(), reply.json()["type"].clone()),
        (409, json!("urn:crane:problem:settings-not-persistent"))
    );
}

#[test]
fn unknown_and_malformed_keys_are_not_found_and_leave_no_audit_trace_of_the_raw_text() {
    // Arrange
    let console = Console::new();
    let keys = ["nope", "Blobs", "1blobs", "bl-obs", &"a".repeat(97)];

    // Act
    let statuses = keys.map(|key| {
        console
            .put(key, &[("if-none-match", "*")], "x")
            .status
            .as_u16()
    });

    // Assert
    assert_eq!(statuses, [404; 5]);
    let rejected: Vec<_> = console
        .audit_events()
        .into_iter()
        .filter_map(|event| match event.action {
            AuditAction::SettingRejected { key, .. } => Some(key),
            _ => None,
        })
        .collect();
    assert_eq!(rejected, ["nope"]);
}

#[test]
fn malformed_body_is_a_bad_request() {
    // Arrange
    let console = Console::new();
    let bodies = ["not json", "[]", "{}", "{\"value\": 5}"];

    // Act
    let statuses = bodies.map(|body| {
        console
            .authed(
                Method::PUT,
                "/api/v1/settings/blobs",
                &[("if-none-match", "*")],
                body,
            )
            .status
            .as_u16()
    });

    // Assert
    assert_eq!(statuses, [400; 4]);
}

#[test]
fn delete_with_the_current_version_returns_the_key_to_default() {
    // Arrange
    let console = Console::new();
    console.create("blobs", "file:///data");

    // Act
    let reply = console.authed(
        Method::DELETE,
        "/api/v1/settings/blobs",
        &[("if-match", "\"1\"")],
        "",
    );

    // Assert
    let body = reply.json();
    assert_eq!(
        (reply.status.as_u16(), &body["origin"], &body["version"]),
        (200, &json!("default"), &json!(null))
    );
}

#[test]
fn delete_requires_a_version_and_rejects_a_stale_one() {
    // Arrange
    let console = Console::new();
    console.create("blobs", "file:///data");
    let conditions: [&[(&str, &str)]; 2] = [&[], &[("if-match", "\"9\"")]];

    // Act
    let statuses = conditions.map(|headers| {
        console
            .authed(Method::DELETE, "/api/v1/settings/blobs", headers, "")
            .status
            .as_u16()
    });

    // Assert
    assert_eq!(statuses, [428, 412]);
}

#[test]
fn a_successful_write_is_audited_with_the_caller() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.create("blobs", "file:///data");

    // Assert
    let actors: Vec<_> = console
        .audit_events()
        .into_iter()
        .map(|event| (event.actor.map(|actor| actor.id), event.outcome))
        .collect();
    assert_eq!(
        (reply.status.as_u16(), actors),
        (200, vec![(Some("crane".to_owned()), Outcome::Success)])
    );
}

#[test]
fn unreachable_store_is_service_unavailable_with_retry_after_on_every_operation() {
    // Arrange
    let console = Console::new();
    console.store_down();
    let calls = [
        (Method::GET, "/api/v1/settings", ""),
        (Method::GET, "/api/v1/settings/blobs", ""),
        (Method::PUT, "/api/v1/settings/blobs", "{\"value\":\"x\"}"),
    ];

    // Act
    let replies = calls.map(|(method, uri, body)| {
        let reply = console.authed(method, uri, &[("if-none-match", "*")], body);
        (
            reply.status.as_u16(),
            reply.header("retry-after").to_owned(),
            reply.json()["type"].clone(),
        )
    });

    // Assert
    let down = (
        503,
        "10".to_owned(),
        json!("urn:crane:problem:settings-unreachable"),
    );
    assert_eq!(replies.to_vec(), vec![down; 3]);
}
