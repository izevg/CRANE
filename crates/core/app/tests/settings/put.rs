//! Запись: успех и каждый отказ оставляют по одному событию, сырое значение в журнал не попадает.

use crane_app::settings::SettingError;
use crane_domain::audit::{AuditAction, Outcome, SettingChange};
use crane_domain::settings::Unreadable;
use crane_ports::settings::SettingsError;
use futures_executor::block_on;

use crate::common::caller;
use crate::fixtures::{
    FailingStore, HIDDEN, LIMIT, LOCKED, LOCKED_VAR, TOKEN, UnreadableStore, seeded, version, world,
};
use crate::{actions_with_codes, event, rejected};

const RAW: &str = "tok_RAWVALUE";

fn changed(key: &str, ver: u64, change: SettingChange) -> AuditAction {
    AuditAction::SettingChanged {
        key: key.to_owned(),
        version: version(ver),
        change,
    }
}

#[test]
fn put_creates_plain_setting_and_audits_change_without_old_value() {
    // Arrange
    let (settings, audit) = world(seeded(&[]), true, &[]);

    // Act
    let result = block_on(settings.put(&caller(), LIMIT, "10".to_owned(), None));

    // Assert
    assert_eq!(
        (result.unwrap(), audit.events()),
        (
            version(1),
            vec![event(
                changed(
                    LIMIT,
                    1,
                    SettingChange::Plain {
                        old: None,
                        new: "10".to_owned()
                    }
                ),
                Outcome::Success
            )]
        )
    );
}

#[test]
fn put_update_audits_old_and_new_plain_values() {
    // Arrange
    let (settings, audit) = world(seeded(&[(LIMIT, "10")]), true, &[(LIMIT, 1)]);

    // Act
    let result = block_on(settings.put(&caller(), LIMIT, "20".to_owned(), Some(version(1))));

    // Assert
    assert_eq!(
        (result.unwrap(), audit.events()),
        (
            version(2),
            vec![event(
                changed(
                    LIMIT,
                    2,
                    SettingChange::Plain {
                        old: Some("10".to_owned()),
                        new: "20".to_owned()
                    }
                ),
                Outcome::Success
            )]
        )
    );
}

#[test]
fn put_secret_audits_no_values_and_marks_readable_row_as_not_unreadable() {
    // Arrange
    let (settings, audit) = world(seeded(&[(TOKEN, "old")]), true, &[(TOKEN, 1)]);

    // Act
    block_on(settings.put(&caller(), TOKEN, RAW.to_owned(), Some(version(1)))).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            changed(
                TOKEN,
                2,
                SettingChange::Secret {
                    replaced_unreadable: false
                }
            ),
            Outcome::Success
        )]
    );
}

#[test]
fn put_secret_over_unreadable_row_flags_replaced_unreadable() {
    // Arrange
    let (settings, audit) = world(UnreadableStore(Unreadable::MissingKey), true, &[]);

    // Act
    block_on(settings.put(&caller(), TOKEN, RAW.to_owned(), Some(version(3)))).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            changed(
                TOKEN,
                4,
                SettingChange::Secret {
                    replaced_unreadable: true
                }
            ),
            Outcome::Success
        )]
    );
}

#[test]
fn put_rejections_are_audited_once_with_stable_code() {
    // Arrange
    let cases = [
        ("unknown", seeded(&[]), true, "nope", None),
        ("hidden", seeded(&[]), true, HIDDEN, None),
        ("locked", seeded(&[]), true, LOCKED, None),
        (
            "locked wins over not persistent",
            seeded(&[]),
            false,
            LOCKED,
            None,
        ),
        ("not persistent", seeded(&[]), false, LIMIT, None),
        ("invalid", seeded(&[]), true, LIMIT, None),
        ("stale", seeded(&[]), true, LIMIT, Some(version(1))),
    ]
    .map(|(name, store, persistent, key, expected)| {
        let raw = if name == "invalid" { "" } else { RAW };
        let (settings, audit) = world(store, persistent, &[]);
        (settings, audit, key, raw, expected)
    });

    // Act
    let actual = cases.map(|(settings, audit, key, raw, expected)| {
        let result = block_on(settings.put(&caller(), key, raw.to_owned(), expected));
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
        ("setting-locked", rejected(LOCKED, Some(LOCKED_VAR))),
        ("settings-not-persistent", rejected(LIMIT, None)),
        ("setting-invalid", rejected(LIMIT, None)),
        ("setting-conflict", rejected(LIMIT, None)),
    ]
    .map(|(code, action)| (Err(code), vec![(action, Some(code))]));
    assert_eq!(actual, expected);
}

#[test]
fn rejection_events_never_contain_raw_value() {
    // Arrange
    let cases = [
        ("nope", true, None),
        (HIDDEN, true, None),
        (LOCKED, true, None),
        (LIMIT, false, None),
        (LIMIT, true, Some(version(1))),
    ]
    .map(|(key, persistent, expected)| {
        let (settings, audit) = world(seeded(&[]), persistent, &[]);
        (settings, audit, key, expected)
    });

    // Act
    let leaked = cases.map(|(settings, audit, key, expected)| {
        let _ = block_on(settings.put(&caller(), key, RAW.to_owned(), expected));
        format!("{:?}", audit.events()).contains("RAWVALUE")
    });

    // Assert
    assert_eq!(leaked, [false; 5]);
}

#[test]
fn put_when_not_persistent_leaves_state_unchanged() {
    // Arrange
    let (settings, _) = world(seeded(&[]), false, &[]);

    // Act
    let result = block_on(settings.put(&caller(), LIMIT, "10".to_owned(), None));

    // Assert
    let list = block_on(settings.view()).unwrap();
    assert_eq!(
        (
            matches!(result, Err(SettingError::NotPersistent)),
            list.settings[0].stored.is_none()
        ),
        (true, true)
    );
}

#[test]
fn store_failures_map_to_codes_and_are_audited() {
    // Arrange
    let makers: [fn() -> SettingsError; 3] = [
        || SettingsError::SecretsUnavailable,
        || SettingsError::Unreachable("db down".into()),
        || SettingsError::Backend("boom".into()),
    ];
    let cases = makers.map(|make| {
        let (settings, audit) = world(FailingStore(make), true, &[]);
        (settings, audit)
    });

    // Act
    let actual = cases.map(|(settings, audit)| {
        let result = block_on(settings.put(&caller(), TOKEN, RAW.to_owned(), None));
        (
            result.map_err(|error| error.code()),
            actions_with_codes(&audit),
        )
    });

    // Assert
    let expected = [
        "secrets-unavailable",
        "settings-unreachable",
        "settings-backend",
    ]
    .map(|code| (Err(code), vec![(rejected(TOKEN, None), Some(code))]));
    assert_eq!(actual, expected);
}

#[test]
fn failure_reason_does_not_leak_backend_source() {
    // Arrange
    let store = FailingStore(|| SettingsError::Backend("postgres://user:pw@host".into()));
    let (settings, audit) = world(store, true, &[]);

    // Act
    let _ = block_on(settings.put(&caller(), LIMIT, "1".to_owned(), None));

    // Assert
    let reasons: Vec<String> = audit
        .events()
        .into_iter()
        .filter_map(|event| match event.outcome {
            Outcome::Failure { reason, .. } => Some(reason),
            Outcome::Success => None,
        })
        .collect();
    assert!(!reasons.iter().any(|reason| reason.contains("pw@host")));
}
