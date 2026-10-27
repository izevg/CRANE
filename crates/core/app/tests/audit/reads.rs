//! Чтения не пишут в журнал.

use crane_domain::registry::{Format, VersionState};
use futures_executor::block_on;

use crate::common::{HOSTED, Seed, asset_path, caller, coordinate, name};

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
