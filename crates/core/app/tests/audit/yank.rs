//! Аудит yank и unyank.

use crane_domain::audit::{AuditAction, Outcome};
use crane_domain::registry::VersionState;
use futures_executor::block_on;

use crate::common::{Seed, caller, coordinate};

use super::{event, failure};

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
