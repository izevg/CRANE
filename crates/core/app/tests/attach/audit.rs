//! Аудит прикрепления: одно событие с путём файла, `Reserve` только у файла, захватившего имя.

use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::{Format, Namespace, NamespacePattern, VersionState, WorkspaceId};
use futures_executor::block_on;

use crate::common::faults::BlindCatalog;
use crate::common::{Seed, attach_request, caller, coordinate, name, principal, request_context};

use super::{FIRST, HELLO, SECOND};

fn event(action: AuditAction, outcome: Outcome) -> AuditEvent {
    AuditEvent {
        actor: Some(principal()),
        request: request_context(),
        action,
        outcome,
    }
}

fn published_file(path: &str) -> AuditAction {
    AuditAction::PublishFile {
        coordinate: coordinate("foo", "1.0.0"),
        path: path.to_owned(),
    }
}

#[test]
fn first_file_of_new_name_records_publish_file_and_reserve_events() {
    // Arrange
    let seed = Seed::new();
    let audit = seed.audit();
    let registry = seed.registry();
    let request = attach_request("foo", "1.0.0", FIRST, &HELLO, Vec::new());

    // Act
    block_on(registry.attach(&caller(), request)).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [
            event(published_file(FIRST), Outcome::Success),
            event(
                AuditAction::Reserve(Namespace {
                    format: Format::Cargo,
                    pattern: NamespacePattern::exact(&name("foo")),
                    workspace: WorkspaceId::DEFAULT,
                }),
                Outcome::Success
            ),
        ]
    );
}

#[test]
fn next_file_records_one_event_with_its_own_path() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    let audit = seed.audit();
    let registry = seed.registry();
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());

    // Act
    block_on(registry.attach(&caller(), request)).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(published_file(SECOND), Outcome::Success)]
    );
}

#[test]
fn refused_file_records_one_failure_event_with_its_path() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    let audit = seed.audit();
    let registry = seed.registry();
    let request = attach_request("foo", "1.0.0", FIRST, &HELLO, Vec::new());

    // Act
    let _ = block_on(registry.attach(&caller(), request));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            published_file(FIRST),
            Outcome::Failure {
                reason: "файл версии уже существует".to_owned(),
                code: None
            }
        )]
    );
}

#[test]
fn race_loser_does_not_record_reserve() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    let audit = seed.audit();
    let registry = seed.registry_with(BlindCatalog::once, |blobs| blobs);
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());

    // Act
    block_on(registry.attach(&caller(), request)).unwrap();

    // Assert
    assert_eq!(
        audit.events(),
        [event(published_file(SECOND), Outcome::Success)],
        "имя захватил победитель гонки, не проигравший"
    );
}
