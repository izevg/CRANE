//! Устаревание версий и dist-теги: допустимость по состоянию, перенос тега, чтение пакета.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::{HOSTED, Seed, caller, coordinate, name, package_ref};
use crane_app::registry::{PackageView, Registry, RegistryError};
use crane_domain::registry::{Format, VersionState};
use crane_memory::{MemoryBlobStore, MemoryCatalog};
use futures_executor::block_on;

type TestRegistry = Registry<MemoryCatalog, MemoryBlobStore>;

fn view(registry: &TestRegistry) -> PackageView {
    block_on(registry.package(&caller(), Format::Cargo, HOSTED, &name("foo"))).unwrap()
}

fn tag_names(view: &PackageView) -> Vec<&str> {
    view.tags.iter().map(|(tag, _)| tag.as_str()).collect()
}

fn unavailable_state(result: &Result<(), RegistryError>) -> Option<VersionState> {
    match result {
        Err(RegistryError::Unavailable { state }) => Some(*state),
        _ => None,
    }
}

#[test]
fn deprecate_sets_message_visible_to_clients() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();

    // Act
    let result = block_on(registry.set_deprecated(
        &caller(),
        &coordinate("foo", "1.0.0"),
        Some("use 2.x".to_owned()),
    ));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(
        view(&registry)
            .versions
            .into_iter()
            .map(|v| v.deprecated)
            .collect::<Vec<_>>(),
        [Some("use 2.x".to_owned())]
    );
}

#[test]
fn undeprecate_clears_message() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();
    let target = coordinate("foo", "1.0.0");
    block_on(registry.set_deprecated(&caller(), &target, Some("old".to_owned()))).unwrap();

    // Act
    let result = block_on(registry.set_deprecated(&caller(), &target, None));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(
        view(&registry)
            .versions
            .into_iter()
            .map(|v| v.deprecated)
            .collect::<Vec<_>>(),
        [None]
    );
}

#[test]
fn deprecate_of_deleted_version_is_unavailable() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Deleted, b"x");
    let registry = seed.registry();

    // Act
    let result = block_on(registry.set_deprecated(
        &caller(),
        &coordinate("foo", "1.0.0"),
        Some("old".to_owned()),
    ));

    // Assert
    assert_eq!(unavailable_state(&result), Some(VersionState::Deleted));
}

#[test]
fn deprecate_of_unknown_version_is_not_found() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let result = block_on(registry.set_deprecated(&caller(), &coordinate("foo", "1.0.0"), None));

    // Assert
    assert!(matches!(result, Err(RegistryError::VersionNotFound)));
}

#[test]
fn tag_on_released_version_is_listed_with_that_version() {
    // Arrange
    let seed = Seed::new();
    let record = seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();

    // Act
    let result = block_on(registry.set_tag(&caller(), &coordinate("foo", "1.0.0"), "latest"));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(view(&registry).tags, [("latest".to_owned(), record.id)]);
}

#[test]
fn tag_moves_to_other_version() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"a");
    let second = seed.add_version("foo", "2.0.0", VersionState::Released, b"b");
    let registry = seed.registry();
    block_on(registry.set_tag(&caller(), &coordinate("foo", "1.0.0"), "latest")).unwrap();

    // Act
    let result = block_on(registry.set_tag(&caller(), &coordinate("foo", "2.0.0"), "latest"));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(view(&registry).tags, [("latest".to_owned(), second.id)]);
}

#[test]
fn tag_on_version_that_is_not_released_is_unavailable() {
    // Arrange
    let states = [
        VersionState::Quarantined,
        VersionState::Blocked,
        VersionState::Deleted,
    ];

    // Act
    let actual = states.map(|state| {
        let seed = Seed::new();
        seed.add_version("foo", "1.0.0", state, b"x");
        let registry = seed.registry();
        unavailable_state(&block_on(registry.set_tag(
            &caller(),
            &coordinate("foo", "1.0.0"),
            "latest",
        )))
    });

    // Assert
    assert_eq!(states.map(Some), actual);
}

#[test]
fn remove_tag_drops_only_that_tag() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();
    let target = coordinate("foo", "1.0.0");
    block_on(registry.set_tag(&caller(), &target, "latest")).unwrap();
    block_on(registry.set_tag(&caller(), &target, "next")).unwrap();

    // Act
    let result = block_on(registry.remove_tag(&caller(), &package_ref(HOSTED, "foo"), "next"));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(tag_names(&view(&registry)), ["latest"]);
}

#[test]
fn remove_absent_tag_is_tag_not_found() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let registry = seed.registry();

    // Act
    let result = block_on(registry.remove_tag(&caller(), &package_ref(HOSTED, "foo"), "next"));

    // Assert
    assert!(matches!(result, Err(RegistryError::TagNotFound)));
}

#[test]
fn remove_tag_in_unknown_repository_is_repository_not_found() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let result = block_on(registry.remove_tag(&caller(), &package_ref("nope", "foo"), "latest"));

    // Assert
    assert!(matches!(result, Err(RegistryError::RepositoryNotFound)));
}

#[test]
fn package_view_hides_unserved_versions_but_keeps_all_tags() {
    // Arrange
    let seed = Seed::new();
    let released = seed.add_version("foo", "1.0.0", VersionState::Released, b"a");
    seed.add_version("foo", "2.0.0", VersionState::Quarantined, b"b");
    let registry = seed.registry();
    block_on(registry.set_tag(&caller(), &coordinate("foo", "1.0.0"), "latest")).unwrap();

    // Act
    let package = view(&registry);

    // Assert
    assert_eq!(
        (
            package.versions.iter().map(|v| v.id).collect::<Vec<_>>(),
            package.tags
        ),
        (vec![released.id], vec![("latest".to_owned(), released.id)])
    );
}

#[test]
fn package_view_of_unknown_package_is_empty() {
    // Arrange
    let registry = Seed::new().registry();

    // Act
    let package = view(&registry);

    // Assert
    assert_eq!((package.versions.len(), package.tags.len()), (0, 0));
}
