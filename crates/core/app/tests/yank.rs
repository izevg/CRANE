//! Yank и unyank: флаг ортогонален состоянию, tombstone неизменяем.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::{HOSTED, InstrumentedCatalog, Seed, coordinate, name, yank_calls, yank_log};
use crane_app::registry::{Registry, RegistryError};
use crane_domain::registry::{Format, VersionState};
use crane_ports::registry::{BlobStore, Catalog};
use futures_executor::block_on;

fn yanked_flags<C: Catalog, B: BlobStore>(registry: &Registry<C, B>) -> Vec<bool> {
    block_on(registry.versions(Format::Cargo, HOSTED, &name("foo")))
        .unwrap()
        .iter()
        .map(|v| v.yanked)
        .collect()
}

#[test]
fn yank_then_unyank_toggles_flag_visible_to_clients() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();
    let target = coordinate("foo", "1.0.0");

    // Act
    block_on(registry.set_yanked(&target, true)).unwrap();
    let after_yank = yanked_flags(&registry);
    block_on(registry.set_yanked(&target, false)).unwrap();
    let after_unyank = yanked_flags(&registry);

    // Assert
    assert_eq!(after_yank, [true], "yank виден в списке версий");
    assert_eq!(after_unyank, [false], "unyank снимает флаг");
}

#[test]
fn yank_is_idempotent() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();
    let target = coordinate("foo", "1.0.0");

    // Act
    let first = block_on(registry.set_yanked(&target, true));
    let second = block_on(registry.set_yanked(&target, true));

    // Assert
    assert!(
        first.is_ok() && second.is_ok(),
        "повторный yank не ошибка: {first:?} {second:?}"
    );
    assert_eq!(yanked_flags(&registry), [true], "версия остаётся yanked");
}

#[test]
fn yank_of_deleted_version_is_unavailable_and_does_not_touch_catalog() {
    // Arrange
    let seed = Seed::new();
    seed.add_moved("foo", "1.0.0", VersionState::Deleted, b"x");
    let log = yank_log();
    let registry = seed.registry_with(|c| InstrumentedCatalog::spying(c, log.clone()), |b| b);

    // Act
    let error = block_on(registry.set_yanked(&coordinate("foo", "1.0.0"), true)).unwrap_err();

    // Assert
    assert!(
        matches!(
            error,
            RegistryError::Unavailable {
                state: VersionState::Deleted
            }
        ),
        "tombstone терминален: {error:?}"
    );
    assert!(yank_calls(&log).is_empty(), "флаг tombstone не меняется");
}

#[test]
fn blocked_version_can_be_yanked_because_flag_is_orthogonal() {
    // Arrange
    let seed = Seed::new();
    seed.add_moved("foo", "1.0.0", VersionState::Blocked, b"x");
    let log = yank_log();
    let registry = seed.registry_with(|c| InstrumentedCatalog::spying(c, log.clone()), |b| b);

    // Act
    let result = block_on(registry.set_yanked(&coordinate("foo", "1.0.0"), true));

    // Assert
    assert!(result.is_ok(), "yank не зависит от состояния: {result:?}");
    assert_eq!(yank_calls(&log), [true], "флаг записан в каталог");
}

#[test]
fn yank_of_unknown_version_is_not_found() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let error = block_on(registry.set_yanked(&coordinate("foo", "1.0.0"), true)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::VersionNotFound), "{error:?}");
}
