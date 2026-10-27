//! Аудит: каждая мутация даёт ровно одно событие, чтения не пишут в журнал.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::time::Duration;

use common::{
    HOSTED, RecordingBlobs, Seed, asset_path, caller, commit_log, coordinate, fixed_now, name,
    package_ref, principal, publish_request, request_context,
};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::{DeletePolicy, Format, VersionState};
use futures_executor::block_on;

const HELLO: [&[u8]; 1] = [b"hello"];

fn event(action: AuditAction, outcome: Outcome) -> AuditEvent {
    AuditEvent {
        actor: Some(principal()),
        request: request_context(),
        action,
        outcome,
    }
}

fn failure(reason: &str) -> Outcome {
    Outcome::Failure {
        reason: reason.to_owned(),
    }
}

#[test]
fn successful_publish_records_one_success_event() {
    // Arrange
    let seed = Seed::new();
    let audit = seed.audit();
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    block_on(registry.publish(&caller(), request)).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Publish(coordinate("foo", "1.0.0")),
            Outcome::Success
        )]
    );
}

#[test]
fn republish_records_one_failure_event() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let _ = block_on(registry.publish(&caller(), request));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Publish(coordinate("foo", "1.0.0")),
            failure("версия уже существует")
        )]
    );
}

#[test]
fn publish_to_unknown_repository_records_one_failure_event() {
    // Arrange
    let seed = Seed::new();
    let audit = seed.audit();
    let registry = seed.registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.coordinate.repository = "nope".to_owned();
    let mut expected = coordinate("foo", "1.0.0");
    expected.repository = "nope".to_owned();

    // Act
    let _ = block_on(registry.publish(&caller(), request));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Publish(expected),
            failure("репозиторий не найден")
        )]
    );
}

#[test]
fn backend_failure_reason_hides_the_source_message() {
    // Arrange
    let seed = Seed::new();
    let audit = seed.audit();
    let log = commit_log();
    let registry = seed.registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::failing_commit(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, Vec::new());

    // Act
    let _ = block_on(registry.publish(&caller(), request));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Publish(coordinate("foo", "1.0.0")),
            failure("ошибка бэкенда")
        )],
        "причина — только Display верхнего уровня, без «хранилище отказало»"
    );
}

#[test]
fn yank_records_one_yank_success_event() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    block_on(registry.set_yanked(&caller(), &coordinate("foo", "1.0.0"), true)).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Yank(coordinate("foo", "1.0.0")),
            Outcome::Success
        )]
    );
}

#[test]
fn unyank_records_one_unyank_success_event() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    block_on(registry.set_yanked(&caller(), &coordinate("foo", "1.0.0"), false)).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Unyank(coordinate("foo", "1.0.0")),
            Outcome::Success
        )]
    );
}

#[test]
fn yank_of_unknown_version_records_one_failure_event() {
    // Arrange
    let seed = Seed::new();
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let _ = block_on(registry.set_yanked(&caller(), &coordinate("foo", "1.0.0"), true));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Yank(coordinate("foo", "1.0.0")),
            failure("версия не найдена")
        )]
    );
}

#[test]
fn fetch_records_no_events() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let _ = block_on(registry.fetch(
        &caller(),
        &coordinate("foo", "1.0.0"),
        &asset_path("foo", "1.0.0"),
    ));

    // Assert
    assert_eq!(audit.events(), []);
}

#[test]
fn versions_records_no_events() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let _ = block_on(registry.versions(&caller(), Format::Cargo, HOSTED, &name("foo")));

    // Assert
    assert_eq!(audit.events(), []);
}

const POLICY: DeletePolicy = DeletePolicy {
    window: Duration::from_secs(100),
};

#[test]
fn delete_records_one_delete_success_event() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    block_on(registry.delete(&caller(), &coordinate("foo", "1.0.0"), &POLICY)).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Delete(coordinate("foo", "1.0.0")),
            Outcome::Success
        )]
    );
}

#[test]
fn delete_outside_window_records_one_failure_event() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry_at(fixed_now() + Duration::from_secs(101));

    // Act
    let _ = block_on(registry.delete(&caller(), &coordinate("foo", "1.0.0"), &POLICY));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Delete(coordinate("foo", "1.0.0")),
            failure("окно удаления закрыто: версия опубликована слишком давно")
        )]
    );
}

#[test]
fn delete_package_records_one_success_event_per_deleted_version() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"a");
    seed.add_version("foo", "2.0.0", VersionState::Released, b"b");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    block_on(registry.delete_package(&caller(), Format::Cargo, HOSTED, &name("foo"), &POLICY))
        .unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [
            event(
                AuditAction::Delete(coordinate("foo", "1.0.0")),
                Outcome::Success
            ),
            event(
                AuditAction::Delete(coordinate("foo", "2.0.0")),
                Outcome::Success
            ),
        ]
    );
}

#[test]
fn delete_package_refused_by_window_records_one_failure_event_for_the_late_version() {
    // Arrange
    let seed = Seed::new();
    let now = fixed_now() + Duration::from_secs(1_000);
    seed.add_version_at("foo", "1.0.0", VersionState::Released, b"a", now);
    seed.add_version_at("foo", "2.0.0", VersionState::Released, b"b", fixed_now());
    let audit = seed.audit();
    let registry = seed.registry_at(now);

    // Act
    let _ =
        block_on(registry.delete_package(&caller(), Format::Cargo, HOSTED, &name("foo"), &POLICY));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Delete(coordinate("foo", "2.0.0")),
            failure("окно удаления закрыто: версия опубликована слишком давно")
        )]
    );
}

#[test]
fn deprecate_records_one_event_with_the_message() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    block_on(registry.set_deprecated(
        &caller(),
        &coordinate("foo", "1.0.0"),
        Some("old".to_owned()),
    ))
    .unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Deprecate {
                coordinate: coordinate("foo", "1.0.0"),
                message: Some("old".to_owned())
            },
            Outcome::Success
        )]
    );
}

#[test]
fn undeprecate_records_one_event_without_message() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();
    let target = coordinate("foo", "1.0.0");
    block_on(registry.set_deprecated(&caller(), &target, Some("old".to_owned()))).unwrap();

    // Act
    block_on(registry.set_deprecated(&caller(), &target, None)).unwrap();

    // Assert
    assert_eq!(
        audit.events().last(),
        Some(&event(
            AuditAction::Deprecate {
                coordinate: target,
                message: None
            },
            Outcome::Success
        ))
    );
}

#[test]
fn tag_records_one_success_event() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"a");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    block_on(registry.set_tag(&caller(), &coordinate("foo", "1.0.0"), "latest")).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Tag {
                coordinate: coordinate("foo", "1.0.0"),
                tag: "latest".to_owned()
            },
            Outcome::Success
        )]
    );
}

#[test]
fn tag_of_not_released_version_records_one_failure_event() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "2.0.0", VersionState::Quarantined, b"b");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let _ = block_on(registry.set_tag(&caller(), &coordinate("foo", "2.0.0"), "next"));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Tag {
                coordinate: coordinate("foo", "2.0.0"),
                tag: "next".to_owned()
            },
            failure("версия недоступна: Quarantined")
        )]
    );
}

#[test]
fn untag_records_failure_when_tag_is_absent() {
    // Arrange
    let seed = Seed::new();
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let _ = block_on(registry.remove_tag(&caller(), &package_ref(HOSTED, "foo"), "latest"));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Untag {
                package: package_ref(HOSTED, "foo"),
                tag: "latest".to_owned()
            },
            failure("тег не найден")
        )]
    );
}

#[test]
fn untag_records_one_success_event_for_existing_tag() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"a");
    let registry_audit = seed.audit();
    let registry = seed.registry();
    block_on(registry.set_tag(&caller(), &coordinate("foo", "1.0.0"), "latest")).unwrap();

    // Act
    block_on(registry.remove_tag(&caller(), &package_ref(HOSTED, "foo"), "latest")).unwrap();

    // Assert
    assert_eq!(
        registry_audit.events().last(),
        Some(&event(
            AuditAction::Untag {
                package: package_ref(HOSTED, "foo"),
                tag: "latest".to_owned()
            },
            Outcome::Success
        ))
    );
}

#[test]
fn package_read_records_no_events() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let _ = block_on(registry.package(&caller(), Format::Cargo, HOSTED, &name("foo")));

    // Assert
    assert_eq!(audit.events(), []);
}
