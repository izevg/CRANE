//! Аудит: каждая мутация даёт ровно одно событие, чтения не пишут в журнал.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::{
    HOSTED, RecordingBlobs, Seed, asset_path, caller, commit_log, coordinate, name, principal,
    publish_request, request_context,
};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::{Format, VersionState};
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
    let request = publish_request("foo", "1.0.0", &HELLO, None);

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
    let request = publish_request("foo", "1.0.0", &HELLO, None);

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
    let mut request = publish_request("foo", "1.0.0", &HELLO, None);
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
    let request = publish_request("foo", "1.0.0", &HELLO, None);

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
