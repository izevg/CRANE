//! Секреты не выходят из консоли ни в одном ответе.

use serde_json::json;

use crane_domain::audit::{AuditAction, SettingChange};

use crate::fixture::{Console, ENV_SECRET, ENV_SECRET_KEY, ENV_SECRET_LOCATOR, UNREADABLE_KEY};

const SECRET: &str = "hunter2-very-secret";

#[test]
fn a_stored_secret_is_never_echoed_by_get_or_list() {
    // Arrange
    let console = Console::new();
    console.create("s3_secret", SECRET);

    // Act
    let texts =
        ["/api/v1/settings/s3_secret", "/api/v1/settings"].map(|uri| console.get(uri).text());

    // Assert
    assert_eq!(texts.map(|text| text.contains(SECRET)), [false, false]);
}

#[test]
fn writing_a_secret_does_not_echo_it_back() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.create("s3_secret", SECRET);

    // Assert
    assert_eq!(
        (reply.status.as_u16(), reply.text().contains(SECRET)),
        (200, false)
    );
}

#[test]
fn a_secret_is_shown_as_set_without_a_value() {
    // Arrange
    let console = Console::new();
    console.create("s3_secret", SECRET);

    // Act
    let reply = console.get("/api/v1/settings/s3_secret");

    // Assert
    let body = reply.json();
    assert_eq!(
        (&body["value"], &body["secret_state"], &body["kind"]),
        (&json!(null), &json!("set"), &json!("secret"))
    );
}

#[test]
fn an_absent_secret_is_unset() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get("/api/v1/settings/s3_secret");

    // Assert
    assert_eq!(reply.json()["secret_state"], "unset");
}

#[test]
fn an_unreadable_secret_reports_why_and_which_keys_are_involved() {
    // Arrange
    let console = Console::with_unreadable_secret();

    // Act
    let reply = console.get(&format!("/api/v1/settings/{UNREADABLE_KEY}"));

    // Assert
    let body = reply.json();
    assert_eq!(
        (&body["secret_state"], &body["unreadable"], &body["value"]),
        (
            &json!("unreadable"),
            &json!({"reason": "wrong_key", "stored_key_id": "k-old", "current_key_id": "k-new"}),
            &json!(null)
        )
    );
}

#[test]
fn an_environment_secret_shows_as_set_and_its_value_stays_hidden() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get(&format!("/api/v1/settings/{ENV_SECRET_KEY}"));

    // Assert
    let body = reply.json();
    assert_eq!(
        (
            &body["origin"],
            &body["secret_state"],
            &body["value"],
            reply.text().contains(ENV_SECRET)
        ),
        (&json!("env"), &json!("set"), &json!(null), false)
    );
}

const SECRET_URL: &str = "valkey://cache-user:cache-pw@cache.internal:6379/0";

#[test]
fn a_stored_secret_url_shows_where_it_points_and_not_how_to_log_in() {
    // Arrange
    let console = Console::new();
    console.create("s3_secret", SECRET_URL);

    // Act
    let reply = console.get("/api/v1/settings/s3_secret");

    // Assert
    let text = reply.text();
    assert_eq!(
        (
            reply.json()["locator"].clone(),
            text.contains("cache-user"),
            text.contains("cache-pw"),
            reply.json()["value"].clone()
        ),
        (
            json!("valkey://cache.internal:6379"),
            false,
            false,
            json!(null)
        )
    );
}

#[test]
fn a_secret_that_is_not_a_url_has_no_locator() {
    // Arrange
    let console = Console::new();
    console.create("s3_secret", SECRET);

    // Act
    let reply = console.get("/api/v1/settings/s3_secret");

    // Assert
    assert_eq!(reply.json()["locator"], json!(null));
}

#[test]
fn an_environment_secret_shows_the_locator_the_schema_gave() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get(&format!("/api/v1/settings/{ENV_SECRET_KEY}"));

    // Assert
    assert_eq!(reply.json()["locator"], json!(ENV_SECRET_LOCATOR));
}

#[test]
fn a_plain_setting_has_no_locator() {
    // Arrange
    let console = Console::new();
    console.create("blobs", "s3://bucket/path");

    // Act
    let reply = console.get("/api/v1/settings/blobs");

    // Assert
    assert_eq!(reply.json()["locator"], json!(null));
}

#[test]
fn writing_a_secret_url_audits_the_address_and_no_credentials() {
    // Arrange
    let console = Console::new();

    // Act
    console.create("s3_secret", SECRET_URL);

    // Assert
    let events = console.audit_events();
    let changes: Vec<_> = events
        .iter()
        .filter_map(|event| match &event.action {
            AuditAction::SettingChanged { change, .. } => Some(change.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        (changes, format!("{events:?}").contains("cache-pw")),
        (
            vec![SettingChange::Secret {
                replaced_unreadable: false,
                old_locator: None,
                new_locator: Some("valkey://cache.internal:6379".to_owned())
            }],
            false
        )
    );
}
