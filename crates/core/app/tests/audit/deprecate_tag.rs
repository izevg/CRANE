//! Аудит deprecate и тегов.

use crane_domain::audit::{AuditAction, Outcome};
use crane_domain::registry::VersionState;
use futures_executor::block_on;

use crate::common::{HOSTED, Seed, caller, coordinate, package_ref};

use super::{event, failure};

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
