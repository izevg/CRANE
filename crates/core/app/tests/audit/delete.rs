//! Аудит удаления версии и пакета.

use std::time::Duration;

use crane_domain::audit::{AuditAction, Outcome};
use crane_domain::registry::{DeletePolicy, Format, VersionState};
use futures_executor::block_on;

use crate::common::{HOSTED, Seed, caller, coordinate, fixed_now, name};

use super::{event, failure};

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
fn delete_package_refused_by_state_records_one_failure_event_for_that_version() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"a");
    seed.add_version("foo", "2.0.0", VersionState::Blocked, b"b");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let _ =
        block_on(registry.delete_package(&caller(), Format::Cargo, HOSTED, &name("foo"), &POLICY));

    // Assert
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Delete(coordinate("foo", "2.0.0")),
            failure("версия недоступна: Blocked")
        )]
    );
}
