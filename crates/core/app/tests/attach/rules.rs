//! Правила use case: состояние версии, тип репозитория, политика формата, защита имени и гонка.

use crane_app::registry::{Refusal, RegistryError};
use crane_domain::registry::VersionState;
use crane_ports::proxy::Presence;
use futures_executor::block_on;

use crate::common::faults::{BlindCatalog, RecordingBlobs, commit_log, committed};
use crate::common::namespace::FakeUpstream;
use crate::common::{PROXY, Seed, attach_request, caller, coordinate_in};

use super::{FIRST, HELLO, SECOND, paths_of_foo};

/// Состояние из `Unavailable`, которым закончилось прикрепление к версии в этом состоянии.
fn unavailable_state(state: VersionState) -> Option<VersionState> {
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", state, b"hello");
    let registry = seed.registry();
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());
    match block_on(registry.attach(&caller(), request)) {
        Err(RegistryError::Unavailable { state }) => Some(state),
        _ => None,
    }
}

#[test]
fn files_are_not_attached_to_versions_that_are_not_released() {
    // Arrange
    let cases = [
        VersionState::Deleted,
        VersionState::Blocked,
        VersionState::Quarantined,
    ];

    // Act
    let actual = cases.map(unavailable_state);

    // Assert
    assert_eq!(cases.map(Some), actual);
}

#[test]
fn attach_to_proxy_repository_is_not_hosted() {
    // Arrange
    let registry = Seed::new().with_proxy().registry();
    let mut request = attach_request("foo", "1.0.0", FIRST, &HELLO, Vec::new());
    request.coordinate = coordinate_in(PROXY, "foo", "1.0.0");

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::NotHosted), "{error:?}");
}

#[test]
fn format_refusal_stops_the_first_file_before_any_storage() {
    // Arrange
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let mut request = attach_request("foo", "1.0.0", FIRST, &HELLO, Vec::new());
    request.metadata = Err(Refusal {
        code: "test-refusal",
        reason: "сторонний реестр".to_owned(),
    });

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(&error, RegistryError::Rejected { code, .. } if *code == "test-refusal"),
        "{error:?}"
    );
    assert!(committed(&log).is_empty(), "отказ политики: CAS не тронут");
    assert!(paths_of_foo(&registry).is_empty(), "версия не создана");
}

#[test]
fn format_refusal_applies_to_files_of_existing_version_too() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"hello");
    let registry = seed.registry();
    let mut request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());
    request.metadata = Err(Refusal {
        code: "test-refusal",
        reason: "сторонний реестр".to_owned(),
    });

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::Rejected { .. }), "{error:?}");
    assert_eq!(paths_of_foo(&registry), [FIRST]);
}

#[test]
fn first_file_with_name_of_public_upstream_is_refused_and_stores_nothing() {
    // Arrange
    let log = commit_log();
    let registry = Seed::new()
        .with_upstream(FakeUpstream::new(Presence::Public))
        .registry_with(
            |catalog| catalog,
            |blobs| RecordingBlobs::recording(blobs, log.clone()),
        );
    let request = attach_request("foo", "1.0.0", FIRST, &HELLO, Vec::new());

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::PublicName), "{error:?}");
    assert!(committed(&log).is_empty(), "отказ имени: CAS не тронут");
}

#[test]
fn file_of_existing_version_does_not_ask_the_public_upstream() {
    // Arrange
    let upstream = FakeUpstream::new(Presence::Public);
    let seed = Seed::new().with_upstream(upstream.clone());
    seed.add_version("foo", "1.0.0", VersionState::Released, b"hello");
    let registry = seed.registry();
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());

    // Act
    let result = block_on(registry.attach(&caller(), request));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(upstream.queries(), 0, "имя уже внутреннее");
}

#[test]
fn losing_the_race_for_a_new_version_retries_once_and_attaches_the_file() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    let log = commit_log();
    let registry = seed.registry_with(BlindCatalog::once, |blobs| {
        RecordingBlobs::recording(blobs, log.clone())
    });
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());

    // Act
    let result = block_on(registry.attach(&caller(), request));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(paths_of_foo(&registry), [FIRST, SECOND]);
    assert_eq!(committed(&log).len(), 1, "тело читается один раз");
}

#[test]
fn race_loser_with_same_file_as_winner_succeeds_without_duplicate() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"hello");
    let registry = seed.registry_with(BlindCatalog::once, |blobs| blobs);
    let request = attach_request("foo", "1.0.0", FIRST, &HELLO, vec![]);

    // Act
    let result = block_on(registry.attach(&caller(), request));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(paths_of_foo(&registry), [FIRST]);
}

#[test]
fn race_loser_with_other_content_under_winners_path_is_asset_exists() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    let registry = seed.registry_with(BlindCatalog::once, |blobs| blobs);
    let request = attach_request("foo", "1.0.0", FIRST, &HELLO, Vec::new());

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::AssetExists), "{error:?}");
}

#[test]
fn race_winner_that_is_not_released_makes_the_file_unavailable() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Blocked, b"first");
    let registry = seed.registry_with(BlindCatalog::once, |blobs| blobs);
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(
            error,
            RegistryError::Unavailable {
                state: VersionState::Blocked
            }
        ),
        "{error:?}"
    );
}

#[test]
fn version_vanishing_after_version_exists_is_a_backend_error() {
    // Arrange
    let seed = Seed::new();
    seed.add_version("foo", "1.0.0", VersionState::Released, b"first");
    let registry = seed.registry_with(BlindCatalog::always, |blobs| blobs);
    let request = attach_request("foo", "1.0.0", SECOND, &HELLO, Vec::new());

    // Act
    let error = block_on(registry.attach(&caller(), request)).unwrap_err();

    // Assert
    assert!(matches!(error, RegistryError::Backend(_)), "{error:?}");
}
