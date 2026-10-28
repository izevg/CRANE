//! Аудит публикации: успех, повтор, неизвестный репозиторий, сбой бэкенда.

use crane_domain::audit::{AuditAction, Outcome};
use crane_domain::registry::{Format, Namespace, NamespacePattern, VersionState, WorkspaceId};
use futures_executor::block_on;

use crate::common::faults::{RecordingBlobs, commit_log};
use crate::common::{Seed, caller, coordinate, name, publish_request};

use super::{event, failure};

const HELLO: [&[u8]; 1] = [b"hello"];

#[test]
fn successful_publish_of_new_name_records_publish_and_reserve_events() {
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
        [
            event(
                AuditAction::Publish(coordinate("foo", "1.0.0")),
                Outcome::Success
            ),
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
