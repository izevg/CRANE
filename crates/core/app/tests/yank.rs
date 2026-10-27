//! Yank и unyank: флаг ортогонален состоянию, tombstone неизменяем.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::{HOSTED, Seed, caller, coordinate, name};
use crane_app::registry::{Registry, RegistryError};
use crane_domain::registry::{Format, VersionState};
use crane_ports::registry::{BlobStore, Catalog};
use futures_executor::block_on;

fn yanked_flags<C: Catalog, B: BlobStore>(registry: &Registry<C, B>) -> Vec<bool> {
    block_on(registry.versions(&caller(), Format::Cargo, HOSTED, &name("foo")))
        .unwrap()
        .iter()
        .map(|v| v.yanked)
        .collect()
}

#[test]
fn yank_sets_flag_visible_to_clients() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();

    // Act
    block_on(registry.set_yanked(&caller(), &coordinate("foo", "1.0.0"), true)).unwrap();

    // Assert
    assert_eq!(
        yanked_flags(&registry),
        [true],
        "yank виден в списке версий"
    );
}

#[test]
fn unyank_clears_flag() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();
    let target = coordinate("foo", "1.0.0");
    block_on(registry.set_yanked(&caller(), &target, true)).unwrap();

    // Act
    block_on(registry.set_yanked(&caller(), &target, false)).unwrap();

    // Assert
    assert_eq!(yanked_flags(&registry), [false], "unyank снимает флаг");
}

#[test]
fn repeated_yank_is_not_an_error_and_keeps_flag() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();
    let target = coordinate("foo", "1.0.0");
    block_on(registry.set_yanked(&caller(), &target, true)).unwrap();

    // Act
    let second = block_on(registry.set_yanked(&caller(), &target, true));

    // Assert
    assert!(second.is_ok(), "повторный yank не ошибка: {second:?}");
    assert_eq!(yanked_flags(&registry), [true], "версия остаётся yanked");
}

#[test]
fn yank_of_deleted_version_is_unavailable() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Deleted, b"x");
    let registry = seed.registry();

    // Act
    let error =
        block_on(registry.set_yanked(&caller(), &coordinate("foo", "1.0.0"), true)).unwrap_err();

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
}

#[test]
fn blocked_version_can_be_yanked_because_flag_is_orthogonal() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Blocked, b"x");
    let registry = seed.registry();

    // Act
    let result = block_on(registry.set_yanked(&caller(), &coordinate("foo", "1.0.0"), true));

    // Assert
    assert!(result.is_ok(), "yank не зависит от состояния: {result:?}");
}

#[test]
fn yank_of_unknown_version_is_not_found() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let error =
        block_on(registry.set_yanked(&caller(), &coordinate("foo", "1.0.0"), true)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::VersionNotFound), "{error:?}");
}
