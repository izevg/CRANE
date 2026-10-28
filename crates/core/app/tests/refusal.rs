//! Отказ по правилу формата: публикация отвергается до хранилищ, аудит пишет use case.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::faults::{RecordingBlobs, commit_log, committed};
use common::{HOSTED, Seed, caller, coordinate, name, principal, publish_request, request_context};
use crane_app::registry::{Refusal, RegistryError};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::Format;
use futures_executor::block_on;

const HELLO: [&[u8]; 1] = [b"hello"];
const CODE: &str = "test-refusal";
const REASON: &str = "сторонний реестр";

fn refusal() -> Refusal {
    Refusal {
        code: CODE,
        reason: REASON.to_owned(),
    }
}

#[test]
fn format_policy_refusal_records_one_failure_event_and_stores_nothing() {
    // Arrange
    let seed = Seed::new();
    let audit = seed.audit();
    let log = commit_log();
    let registry = seed.registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.metadata = Err(refusal());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(&error, RegistryError::Rejected { reason, .. } if reason == REASON),
        "{error:?}"
    );
    assert_eq!(
        audit.events(),
        [AuditEvent {
            actor: Some(principal()),
            request: request_context(),
            action: AuditAction::Publish(coordinate("foo", "1.0.0")),
            outcome: Outcome::Failure {
                reason: REASON.to_owned(),
                code: Some(CODE)
            },
        }]
    );
    assert!(committed(&log).is_empty(), "тело не попало в CAS");
    let listed =
        block_on(registry.versions(&caller(), Format::Cargo, HOSTED, &name("foo"))).unwrap();
    assert!(listed.is_empty(), "версии в каталоге нет");
}

#[test]
fn refused_publish_to_unknown_repository_reports_missing_repository_first() {
    // Arrange
    let registry = Seed::new().registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.coordinate.repository = "nope".to_owned();
    request.metadata = Err(refusal());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::RepositoryNotFound),
        "репозиторий проверяется раньше политики формата: {error:?}"
    );
}
