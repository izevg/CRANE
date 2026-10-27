//! Удаление версии и пакета: tombstone только в окне политики, только в hosted.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::time::{Duration, SystemTime};

use common::{HOSTED, PROXY, Seed, caller, coordinate, coordinate_in, fixed_now, name};
use crane_app::registry::RegistryError;
use crane_domain::registry::{DeletePolicy, Format, VersionState};
use futures_executor::block_on;

const WINDOW: Duration = Duration::from_secs(100);
const POLICY: DeletePolicy = DeletePolicy { window: WINDOW };

fn after_publication(by: Duration) -> SystemTime {
    fixed_now() + by
}

fn served_versions(
    registry: &crane_app::registry::Registry<
        crane_memory::MemoryCatalog,
        crane_memory::MemoryBlobStore,
    >,
) -> usize {
    block_on(registry.package(&caller(), Format::Cargo, HOSTED, &name("foo")))
        .unwrap()
        .versions
        .len()
}

#[test]
fn delete_inside_window_hides_version_as_deleted() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry_at(after_publication(Duration::from_secs(10)));
    let target = coordinate("foo", "1.0.0");

    // Act
    let result = block_on(registry.delete(&caller(), &target, &POLICY));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    let fetched = block_on(registry.fetch(&caller(), &target, "foo-1.0.0.crate"));
    assert!(matches!(
        fetched,
        Err(RegistryError::Unavailable {
            state: VersionState::Deleted
        })
    ));
}

#[test]
fn delete_at_exact_window_boundary_succeeds() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry_at(after_publication(WINDOW));

    // Act
    let result = block_on(registry.delete(&caller(), &coordinate("foo", "1.0.0"), &POLICY));

    // Assert
    assert!(result.is_ok(), "возраст ровно в окне допустим: {result:?}");
}

#[test]
fn delete_just_past_window_is_rejected_and_version_stays_served() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry_at(after_publication(WINDOW + Duration::from_secs(1)));

    // Act
    let result = block_on(registry.delete(&caller(), &coordinate("foo", "1.0.0"), &POLICY));

    // Assert
    assert!(matches!(result, Err(RegistryError::DeleteWindowClosed)));
    assert_eq!(served_versions(&registry), 1);
}

#[test]
fn repeated_delete_is_idempotent_even_after_window_closed() {
    // Arrange
    let seed = Seed::new();
    seed.add_version_at("foo", "1.0.0", VersionState::Deleted, b"x", fixed_now());
    let registry = seed.registry_at(after_publication(WINDOW * 10));

    // Act
    let result = block_on(registry.delete(&caller(), &coordinate("foo", "1.0.0"), &POLICY));

    // Assert
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn delete_in_proxy_repository_is_not_hosted() {
    // Arrange
    let registry = Seed::new().with_proxy().registry();

    // Act
    let result =
        block_on(registry.delete(&caller(), &coordinate_in(PROXY, "foo", "1.0.0"), &POLICY));

    // Assert
    assert!(matches!(result, Err(RegistryError::NotHosted)));
}

#[test]
fn delete_of_blocked_version_is_unavailable() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Blocked, b"x");
    let registry = seed.registry();

    // Act
    let result = block_on(registry.delete(&caller(), &coordinate("foo", "1.0.0"), &POLICY));

    // Assert
    assert!(matches!(
        result,
        Err(RegistryError::Unavailable {
            state: VersionState::Blocked
        })
    ));
}

#[test]
fn delete_of_unknown_version_is_not_found() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let result = block_on(registry.delete(&caller(), &coordinate("foo", "1.0.0"), &POLICY));

    // Assert
    assert!(matches!(result, Err(RegistryError::VersionNotFound)));
}

#[test]
fn delete_package_deletes_every_version() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"a");
    seed.add_version("foo", "2.0.0", VersionState::Released, b"b");
    let registry = seed.registry();

    // Act
    let result =
        block_on(registry.delete_package(&caller(), Format::Cargo, HOSTED, &name("foo"), &POLICY));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(served_versions(&registry), 0);
}

#[test]
fn delete_package_with_a_version_outside_window_changes_nothing() {
    // Arrange
    let seed = Seed::new();
    let now = after_publication(Duration::from_secs(1_000));
    seed.add_version_at("foo", "1.0.0", VersionState::Released, b"a", now - WINDOW);
    seed.add_version_at(
        "foo",
        "2.0.0",
        VersionState::Released,
        b"b",
        now - WINDOW - Duration::from_secs(1),
    );
    let registry = seed.registry_at(now);

    // Act
    let result =
        block_on(registry.delete_package(&caller(), Format::Cargo, HOSTED, &name("foo"), &POLICY));

    // Assert
    assert!(matches!(result, Err(RegistryError::DeleteWindowClosed)));
    assert_eq!(served_versions(&registry), 2);
}

#[test]
fn delete_package_skips_deleted_versions() {
    // Arrange
    let seed = Seed::new();
    let now = after_publication(Duration::from_secs(1_000));
    seed.add_version_at("foo", "1.0.0", VersionState::Deleted, b"a", fixed_now());
    seed.add_version_at("foo", "2.0.0", VersionState::Released, b"b", now);
    let registry = seed.registry_at(now);

    // Act
    let result =
        block_on(registry.delete_package(&caller(), Format::Cargo, HOSTED, &name("foo"), &POLICY));

    // Assert
    assert!(
        result.is_ok(),
        "версия вне окна, но уже удалённая, не мешает: {result:?}"
    );
}

#[test]
fn delete_package_of_unknown_package_is_not_found() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let result =
        block_on(registry.delete_package(&caller(), Format::Cargo, HOSTED, &name("foo"), &POLICY));

    // Assert
    assert!(matches!(result, Err(RegistryError::VersionNotFound)));
}

#[test]
fn delete_package_in_proxy_repository_is_not_hosted() {
    // Arrange
    let registry = Seed::new().with_proxy().registry();

    // Act
    let result =
        block_on(registry.delete_package(&caller(), Format::Cargo, PROXY, &name("foo"), &POLICY));

    // Assert
    assert!(matches!(result, Err(RegistryError::NotHosted)));
}
