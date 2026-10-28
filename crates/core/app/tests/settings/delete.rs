//! Удаление: проверка разбором, аудит успеха и отказа.

use crane_app::settings::SettingError;
use crane_domain::audit::{AuditAction, Outcome};
use futures_executor::block_on;

use crate::common::caller;
use crate::fixtures::{HIDDEN, LIMIT, LOCKED, LOCKED_VAR, REQUIRED, TOKEN, seeded, version, world};
use crate::{actions_with_codes, event, rejected};

#[test]
fn delete_removes_row_and_audits_it() {
    // Arrange
    let (settings, audit) = world(seeded(&[(LIMIT, "10")]), true, &[(LIMIT, 1)]);

    // Act
    let result = block_on(settings.delete(&caller(), LIMIT, version(1)));

    // Assert
    let list = block_on(settings.view()).unwrap();
    assert_eq!(
        (
            result.is_ok(),
            list.settings[0].stored.is_none(),
            audit.events()
        ),
        (
            true,
            true,
            vec![event(
                AuditAction::SettingDeleted {
                    key: LIMIT.to_owned(),
                    secret: false
                },
                Outcome::Success
            )]
        )
    );
}

#[test]
fn delete_marks_secret_key_in_event() {
    // Arrange
    let (settings, audit) = world(seeded(&[(TOKEN, "tok")]), true, &[(TOKEN, 1)]);

    // Act
    block_on(settings.delete(&caller(), TOKEN, version(1))).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::SettingDeleted {
                key: TOKEN.to_owned(),
                secret: true
            },
            Outcome::Success
        )]
    );
}

#[test]
fn delete_that_breaks_section_is_rejected_and_row_stays() {
    // Arrange
    let (settings, audit) = world(seeded(&[(REQUIRED, "x")]), true, &[(REQUIRED, 1)]);

    // Act
    let result = block_on(settings.delete(&caller(), REQUIRED, version(1)));

    // Assert
    let list = block_on(settings.view()).unwrap();
    assert_eq!(
        (
            matches!(result, Err(SettingError::Invalid(_))),
            list.settings[3].stored.is_some(),
            actions_with_codes(&audit)
        ),
        (
            true,
            true,
            vec![(rejected(REQUIRED, None), Some("setting-invalid"))]
        )
    );
}

#[test]
fn delete_rejections_are_audited_once_with_stable_code() {
    // Arrange
    let cases = [
        ("nope", true, version(1)),
        (HIDDEN, true, version(1)),
        (LOCKED, true, version(1)),
        (LIMIT, false, version(1)),
        (LIMIT, true, version(7)),
    ]
    .map(|(key, persistent, expected)| {
        let (settings, audit) = world(seeded(&[(LIMIT, "10")]), persistent, &[(LIMIT, 1)]);
        (settings, audit, key, expected)
    });

    // Act
    let actual = cases.map(|(settings, audit, key, expected)| {
        let result = block_on(settings.delete(&caller(), key, expected));
        (
            result.map_err(|error| error.code()),
            actions_with_codes(&audit),
        )
    });

    // Assert
    let expected = [
        ("setting-unknown", rejected("nope", None)),
        ("setting-unknown", rejected(HIDDEN, None)),
        ("setting-locked", rejected(LOCKED, Some(LOCKED_VAR))),
        ("settings-not-persistent", rejected(LIMIT, None)),
        ("setting-conflict", rejected(LIMIT, None)),
    ]
    .map(|(code, action)| (Err(code), vec![(action, Some(code))]));
    assert_eq!(actual, expected);
}
