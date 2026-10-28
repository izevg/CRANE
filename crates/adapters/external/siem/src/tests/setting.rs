#![allow(
    clippy::expect_used,
    reason = "в тестах падение expect — это падение теста"
)]

use crane_domain::access::PrincipalKind;
use crane_domain::audit::{AuditAction, OnboardingCredential, Outcome, SettingChange};
use crane_domain::settings::Version as SettingVersion;
use uuid::Uuid;

use super::tests::{event, failure, principal, record};
use super::{SettingFields, ocsf};

fn setting_changed(change: SettingChange) -> AuditAction {
    AuditAction::SettingChanged {
        key: "blobs".to_owned(),
        version: SettingVersion::FIRST,
        change,
    }
}

#[test]
fn setting_and_onboarding_actions_map_to_api_activity_without_resource() {
    // Arrange
    let robot = Some(principal(PrincipalKind::Robot));
    let cases = [
        setting_changed(SettingChange::Secret {
            replaced_unreadable: false,
        }),
        AuditAction::SettingRejected {
            key: "blobs".to_owned(),
            env_var: Some("CRANE_BLOBS".to_owned()),
        },
        AuditAction::OnboardingStarted,
        AuditAction::OnboardingCompleted {
            via: OnboardingCredential::BootstrapToken,
        },
        AuditAction::OnboardingSkippedPreconfigured,
        AuditAction::OnboardingTokenRotated,
        AuditAction::OnboardingReset,
    ];

    // Act
    let actual = cases.map(|action| {
        let r = record(action, Outcome::Success, robot.clone());
        (r.class_uid, r.activity_id, r.operation, r.resource_uid)
    });

    // Assert
    assert_eq!(
        actual,
        [
            (6003, 3, Some("setting.change"), None),
            (6003, 3, Some("setting.reject"), None),
            (6003, 1, Some("onboarding.start"), None),
            (6003, 3, Some("onboarding.complete"), None),
            (6003, 3, Some("onboarding.skip_preconfigured"), None),
            (6003, 3, Some("onboarding.rotate_token"), None),
            (6003, 3, Some("onboarding.reset"), None),
        ]
    );
}

#[test]
fn rejected_onboarding_token_maps_to_authentication_logon_with_user() {
    // Arrange
    let event = event(AuditAction::OnboardingTokenRejected, failure(), None);

    // Act
    let r = ocsf(&event, Uuid::nil(), 0);

    // Assert
    assert_eq!(
        (
            r.class_uid,
            r.category_uid,
            r.activity_id,
            r.operation,
            r.status_id,
            r.user.is_some(),
            r.actor.is_some(),
        ),
        (3002, 3, 1, None, 2, true, false)
    );
}

#[test]
fn secret_setting_change_carries_no_values() {
    // Arrange
    let event = event(
        setting_changed(SettingChange::Secret {
            replaced_unreadable: false,
        }),
        Outcome::Success,
        Some(principal(PrincipalKind::Human)),
    );

    // Act
    let r = ocsf(&event, Uuid::nil(), 0);

    // Assert
    assert_eq!(
        (
            r.setting.key,
            r.setting.version,
            r.setting.old,
            r.setting.new
        ),
        (Some("blobs".to_owned()), Some(1), None, None)
    );
}

#[test]
fn secret_change_flags_replaced_unreadable_only_when_set() {
    // Arrange
    let cases = [false, true];

    // Act
    let actual = cases.map(|replaced_unreadable| {
        let change = SettingChange::Secret {
            replaced_unreadable,
        };
        let r = record(setting_changed(change), Outcome::Success, None);
        (r.setting.replaced_unreadable, r.setting.old, r.setting.new)
    });

    // Assert
    assert_eq!(actual, [(None, None, None), (Some(true), None, None)]);
}

#[test]
fn plain_setting_change_truncates_values_to_256_chars_on_char_boundary() {
    // Arrange
    let cases = [
        ("я".repeat(255), "я".repeat(255)),
        ("я".repeat(256), "я".repeat(256)),
        ("я".repeat(257), "я".repeat(256)),
    ];

    // Act
    let actual = cases.clone().map(|(raw, _)| {
        let change = SettingChange::Plain {
            old: Some(raw.clone()),
            new: raw,
        };
        let r = record(setting_changed(change), Outcome::Success, None);
        (r.setting.old, r.setting.new)
    });

    // Assert
    let expected = cases.map(|(_, cut)| (Some(cut.clone()), Some(cut)));
    assert_eq!(actual, expected);
}

#[test]
fn plain_setting_creation_has_no_old_value_and_update_has_both() {
    // Arrange
    let cases = [
        SettingChange::Plain {
            old: None,
            new: "b".to_owned(),
        },
        SettingChange::Plain {
            old: Some("a".to_owned()),
            new: "b".to_owned(),
        },
    ];

    // Act
    let actual = cases.map(|change| {
        let r = record(setting_changed(change), Outcome::Success, None);
        (r.setting.old, r.setting.new)
    });

    // Assert
    assert_eq!(
        actual,
        [
            (None, Some("b".to_owned())),
            (Some("a".to_owned()), Some("b".to_owned()))
        ]
    );
}

#[test]
fn deleted_setting_maps_to_delete_activity_and_carries_only_key() {
    // Arrange
    let cases = [true, false].map(|secret| AuditAction::SettingDeleted {
        key: "blobs".to_owned(),
        secret,
    });

    // Act
    let actual = cases.map(|action| {
        let r = record(action, Outcome::Success, None);
        (r.activity_id, r.type_uid, r.operation, r.setting)
    });

    // Assert
    let expected = (
        4,
        600_304,
        Some("setting.delete"),
        SettingFields {
            key: Some("blobs".to_owned()),
            ..SettingFields::default()
        },
    );
    assert_eq!(actual, [expected.clone(), expected]);
}

#[test]
fn rejection_carries_env_var_only_when_key_is_locked() {
    // Arrange
    let cases =
        [Some("CRANE_BLOBS".to_owned()), None].map(|env_var| AuditAction::SettingRejected {
            key: "blobs".to_owned(),
            env_var,
        });

    // Act
    let actual = cases.map(|action| {
        let r = record(action, failure(), None);
        (r.setting.key, r.setting.env_var)
    });

    // Assert
    assert_eq!(
        actual,
        [
            (Some("blobs".to_owned()), Some("CRANE_BLOBS".to_owned())),
            (Some("blobs".to_owned()), None),
        ]
    );
}
