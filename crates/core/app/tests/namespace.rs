//! Защита от dependency confusion: публикация нового имени, `admit_upstream`, reserve.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use common::namespace::{FakeUpstream, Fault, FaultyCatalog, cargo_namespace, other_workspace};
use common::{HOSTED, Seed, caller, coordinate, name, principal, publish_request, request_context};
use crane_app::registry::RegistryError;
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::{Format, VersionState, WorkspaceId};
use crane_ports::proxy::Presence;
use futures_executor::block_on;

const HELLO: [&[u8]; 1] = [b"hello"];

fn event(action: AuditAction, outcome: Outcome) -> AuditEvent {
    AuditEvent {
        actor: Some(principal()),
        request: request_context(),
        action,
        outcome,
    }
}

fn failure(error: &RegistryError) -> Outcome {
    Outcome::Failure {
        reason: error.to_string(),
    }
}

fn shadow(package: &str) -> AuditAction {
    AuditAction::ShadowUpstream {
        format: Format::Cargo,
        package: name(package),
    }
}

fn reserve_action(pattern: &str, workspace: WorkspaceId) -> AuditAction {
    AuditAction::Reserve(cargo_namespace(pattern, workspace))
}

#[test]
fn publish_of_public_name_is_refused_and_stores_nothing() {
    // Arrange
    let seed = Seed::new().with_upstream(FakeUpstream::new(Presence::Public));
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let result = block_on(registry.publish(
        &caller(),
        publish_request("serde", "1.0.0", &HELLO, Vec::new()),
    ));

    // Assert
    let stored = block_on(registry.versions(&caller(), Format::Cargo, HOSTED, &name("serde")));
    assert!(
        matches!(result, Err(RegistryError::PublicName)),
        "{result:?}"
    );
    assert!(matches!(stored.as_deref(), Ok([])), "{stored:?}");
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Publish(coordinate("serde", "1.0.0")),
            failure(&RegistryError::PublicName)
        )]
    );
}

#[test]
fn publish_is_refused_when_upstream_presence_is_unknown() {
    // Arrange
    let registry = Seed::new()
        .with_upstream(FakeUpstream::new(Presence::Unknown))
        .registry();

    // Act
    let result = block_on(registry.publish(
        &caller(),
        publish_request("foo", "1.0.0", &HELLO, Vec::new()),
    ));

    // Assert
    assert!(
        matches!(result, Err(RegistryError::UpstreamUnavailable)),
        "{result:?}"
    );
}

#[test]
fn publish_under_own_namespace_succeeds_without_asking_upstream_and_without_claim() {
    // Arrange
    let upstream = FakeUpstream::new(Presence::Public);
    let seed = Seed::new().with_upstream(upstream.clone());
    seed.add_namespace("acme-*", WorkspaceId::DEFAULT);
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let result = block_on(registry.publish(
        &caller(),
        publish_request("acme-core", "1.0.0", &HELLO, Vec::new()),
    ));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(
        upstream.queries(),
        0,
        "зарезервированное имя upstream не спрашивает"
    );
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Publish(coordinate("acme-core", "1.0.0")),
            Outcome::Success
        )]
    );
}

#[test]
fn publish_under_namespace_of_other_workspace_is_refused() {
    // Arrange
    let seed = Seed::new();
    seed.add_namespace("acme-*", other_workspace());
    let registry = seed.registry();

    // Act
    let result = block_on(registry.publish(
        &caller(),
        publish_request("acme-core", "1.0.0", &HELLO, Vec::new()),
    ));

    // Assert
    assert!(
        matches!(result, Err(RegistryError::NamespaceOwned)),
        "{result:?}"
    );
}

#[test]
fn next_version_of_existing_hosted_name_does_not_consult_upstream() {
    // Arrange
    let upstream = FakeUpstream::new(Presence::Public);
    let seed = Seed::new().with_upstream(upstream.clone());
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let result = block_on(registry.publish(
        &caller(),
        publish_request("foo", "1.1.0", &HELLO, Vec::new()),
    ));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(upstream.queries(), 0);
    assert_eq!(
        audit.events(),
        [event(
            AuditAction::Publish(coordinate("foo", "1.1.0")),
            Outcome::Success
        )]
    );
}

#[test]
fn admit_upstream_refuses_name_covered_by_namespace_and_audits_it() {
    // Arrange
    let seed = Seed::new();
    seed.add_namespace("acme-*", WorkspaceId::DEFAULT);
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let result = block_on(registry.admit_upstream(&caller(), Format::Cargo, &name("acme-core")));

    // Assert
    assert!(matches!(result, Err(RegistryError::Shadowed)), "{result:?}");
    assert_eq!(
        audit.events(),
        [event(
            shadow("acme-core"),
            failure(&RegistryError::Shadowed)
        )]
    );
}

#[test]
fn admit_upstream_refuses_name_hosted_in_another_repository_of_format() {
    // Arrange
    let seed = Seed::new();
    seed.add_package_in_other_hosted("other", "internal-tool");
    let registry = seed.registry();

    // Act
    let result =
        block_on(registry.admit_upstream(&caller(), Format::Cargo, &name("Internal_Tool")));

    // Assert
    assert!(matches!(result, Err(RegistryError::Shadowed)), "{result:?}");
}

#[test]
fn admit_upstream_allows_unknown_name_without_audit() {
    // Arrange
    let seed = Seed::new();
    seed.add_namespace("acme-*", WorkspaceId::DEFAULT);
    seed.add_version("foo", "1.0.0", VersionState::Released, b"x");
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let result = block_on(registry.admit_upstream(&caller(), Format::Cargo, &name("serde")));

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(audit.events(), []);
}

#[test]
fn reserve_of_new_namespace_returns_it_and_audits_success() {
    // Arrange
    let seed = Seed::new();
    let audit = seed.audit();
    let registry = seed.registry();
    let requested = cargo_namespace("acme-*", WorkspaceId::DEFAULT);

    // Act
    let result = block_on(registry.reserve(&caller(), requested.clone()));

    // Assert
    assert_eq!(result.unwrap(), requested);
    assert_eq!(
        audit.events(),
        [event(
            reserve_action("acme-*", WorkspaceId::DEFAULT),
            Outcome::Success
        )]
    );
}

#[test]
fn reserve_of_pattern_owned_by_other_workspace_is_conflict_and_audits_failure() {
    // Arrange
    let seed = Seed::new();
    seed.add_namespace("acme-*", other_workspace());
    let audit = seed.audit();
    let registry = seed.registry();

    // Act
    let result =
        block_on(registry.reserve(&caller(), cargo_namespace("acme-*", WorkspaceId::DEFAULT)));

    // Assert
    assert!(
        matches!(result, Err(RegistryError::NamespaceConflict)),
        "{result:?}"
    );
    assert_eq!(
        audit.events(),
        [event(
            reserve_action("acme-*", WorkspaceId::DEFAULT),
            failure(&RegistryError::NamespaceConflict)
        )]
    );
}

/// Сбой поиска имени не должен превращаться в «имя свободно»: иначе прокси пойдёт
/// за внутренним именем в публичный upstream.
fn admit_with_failing(failing: &'static [Fault]) -> (Result<(), RegistryError>, Vec<AuditEvent>) {
    let seed = Seed::new();
    let audit = seed.audit();
    let registry = seed.registry_with(|inner| FaultyCatalog { inner, failing }, |blobs| blobs);
    let result = block_on(registry.admit_upstream(&caller(), Format::Cargo, &name("serde")));
    (result, audit.events())
}

#[test]
fn admit_upstream_fails_closed_when_namespace_lookup_is_down() {
    // Arrange
    let failing = &[Fault::NamespaceFor];

    // Act
    let (result, events) = admit_with_failing(failing);

    // Assert
    assert!(
        matches!(result, Err(RegistryError::Backend(_))),
        "{result:?}"
    );
    assert!(
        events.iter().all(|e| e.outcome != Outcome::Success),
        "{events:?}"
    );
}

#[test]
fn admit_upstream_fails_closed_when_hosted_lookup_is_down() {
    // Arrange
    let failing = &[Fault::HostedExists];

    // Act
    let (result, events) = admit_with_failing(failing);

    // Assert
    assert!(
        matches!(result, Err(RegistryError::Backend(_))),
        "{result:?}"
    );
    assert!(
        events.iter().all(|e| e.outcome != Outcome::Success),
        "{events:?}"
    );
}
