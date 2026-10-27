#![allow(
    clippy::expect_used,
    reason = "в тестах падение expect — это падение теста"
)]

use crane_domain::access::{RequestContext, RequestId};
use crane_domain::registry::{Coordinate, PackageRef};

use super::*;

fn coord(name: &str, version: &str) -> Coordinate {
    Coordinate {
        format: Format::Cargo,
        repository: "internal".to_owned(),
        package: PackageName::parse(Format::Cargo, name).expect("имя"),
        version: Version::parse(Format::Cargo, version).expect("версия"),
    }
}

fn principal(kind: PrincipalKind) -> Principal {
    Principal {
        id: "crane".to_owned(),
        kind,
    }
}

fn event(action: AuditAction, outcome: Outcome, actor: Option<Principal>) -> AuditEvent {
    AuditEvent {
        actor,
        request: RequestContext {
            request_id: RequestId::parse("req-1").expect("id"),
            trace_id: Some("4bf92f3577b34da6a3ce929d0e0e4736".to_owned()),
            client_ip: "10.0.4.17".parse().ok(),
            user_agent: Some("cargo/1.90".to_owned()),
        },
        action,
        outcome,
    }
}

fn package_ref() -> PackageRef {
    PackageRef {
        format: Format::Cargo,
        repository: "internal".to_owned(),
        package: PackageName::parse(Format::Cargo, "serde").expect("имя"),
    }
}

fn deprecate(message: Option<&str>) -> AuditAction {
    AuditAction::Deprecate {
        coordinate: coord("serde", "1.0.0"),
        message: message.map(str::to_owned),
    }
}

fn failure() -> Outcome {
    Outcome::Failure {
        reason: "причина".to_owned(),
    }
}

fn record(action: AuditAction, outcome: Outcome, actor: Option<Principal>) -> OcsfRecord {
    ocsf(&event(action, outcome, actor), Uuid::nil(), 7)
}

#[test]
fn action_and_outcome_map_to_class_activity_type_status_and_severity() {
    // Arrange
    let robot = Some(principal(PrincipalKind::Robot));
    let c = || coord("serde", "1.0.0");
    let cases = [
        (AuditAction::Authenticate, Outcome::Success, None),
        (AuditAction::Authenticate, failure(), None),
        (AuditAction::Publish(c()), Outcome::Success, robot.clone()),
        (AuditAction::Publish(c()), failure(), robot.clone()),
        (AuditAction::Yank(c()), Outcome::Success, robot.clone()),
        (AuditAction::Unyank(c()), failure(), robot.clone()),
        (AuditAction::Delete(c()), Outcome::Success, robot.clone()),
        (AuditAction::Delete(c()), failure(), robot.clone()),
        (deprecate(Some("old")), Outcome::Success, robot.clone()),
        (deprecate(None), Outcome::Success, robot.clone()),
        (
            AuditAction::Tag {
                coordinate: c(),
                tag: "latest".to_owned(),
            },
            Outcome::Success,
            robot.clone(),
        ),
        (
            AuditAction::Untag {
                package: package_ref(),
                tag: "latest".to_owned(),
            },
            failure(),
            robot,
        ),
    ];

    // Act
    let actual = cases.map(|(action, outcome, actor)| {
        let r = record(action, outcome, actor);
        (
            r.class_uid,
            r.category_uid,
            r.activity_id,
            r.type_uid,
            r.status_id,
            r.severity_id,
            r.operation,
        )
    });

    // Assert
    assert_eq!(
        actual,
        [
            (3002, 3, 1, 300_201, 1, 1, None),
            (3002, 3, 1, 300_201, 2, 2, None),
            (6003, 6, 1, 600_301, 1, 1, Some("publish")),
            (6003, 6, 1, 600_301, 2, 2, Some("publish")),
            (6003, 6, 3, 600_303, 1, 1, Some("yank")),
            (6003, 6, 3, 600_303, 2, 2, Some("unyank")),
            (6003, 6, 4, 600_304, 1, 1, Some("delete")),
            (6003, 6, 4, 600_304, 2, 2, Some("delete")),
            (6003, 6, 3, 600_303, 1, 1, Some("deprecate")),
            (6003, 6, 3, 600_303, 1, 1, Some("undeprecate")),
            (6003, 6, 3, 600_303, 1, 1, Some("tag")),
            (6003, 6, 3, 600_303, 2, 2, Some("untag")),
        ]
    );
}

#[test]
fn failure_carries_reason_and_success_carries_none() {
    // Arrange
    let actor = Some(principal(PrincipalKind::Robot));
    let cases = [failure(), Outcome::Success];

    // Act
    let details = cases.map(|outcome| {
        let action = AuditAction::Publish(coord("serde", "1.0.0"));
        record(action, outcome, actor.clone()).status_detail
    });

    // Assert
    assert_eq!(details, [Some("причина".to_owned()), None]);
}

#[test]
fn purl_encodes_plus_in_version_and_keeps_written_name() {
    // Arrange
    let cases = [
        AuditAction::Publish(coord("serde", "1.0.0")),
        AuditAction::Publish(coord("Foo_Bar", "1.2.3-rc.1+build5")),
    ];

    // Act
    let actual = cases.map(|action| resource(&action).as_ref().map(purl));

    // Assert
    assert_eq!(
        actual,
        [
            Some("pkg:cargo/serde@1.0.0".to_owned()),
            Some("pkg:cargo/Foo_Bar@1.2.3-rc.1%2Bbuild5".to_owned()),
        ]
    );
}

#[test]
fn purl_component_encodes_plus_and_scope_marker() {
    // Arrange
    let cases = ["@scope/pkg", "1.0.0+build", "plain"];

    // Act
    let actual = cases.map(purl_component);

    // Assert
    assert_eq!(actual, ["%40scope/pkg", "1.0.0%2Bbuild", "plain"]);
}

#[test]
fn package_level_action_resource_has_purl_without_version() {
    // Arrange
    let action = AuditAction::Untag {
        package: package_ref(),
        tag: "latest".to_owned(),
    };

    // Act
    let r = ocsf(&event(action, Outcome::Success, None), Uuid::nil(), 0);

    // Assert
    assert_eq!(
        (r.resource_uid.as_deref(), r.repository.as_deref()),
        (Some("pkg:cargo/serde"), Some("internal"))
    );
}

#[test]
fn principal_kind_maps_to_ocsf_user_type() {
    // Arrange
    let kinds = [
        PrincipalKind::Human,
        PrincipalKind::Robot,
        PrincipalKind::Workload,
        PrincipalKind::Agent,
    ];

    // Act
    let actual = kinds.map(|kind| {
        let r = record(
            AuditAction::Yank(coord("serde", "1.0.0")),
            Outcome::Success,
            Some(principal(kind)),
        );
        r.actor.map(|u| (u.type_id, u.type_name))
    });

    // Assert
    let expected = [(1, "User"), (99, "Robot"), (99, "Workload"), (99, "Agent")]
        .map(|(id, name)| Some((id, name.to_owned())));
    assert_eq!(actual, expected);
}

#[test]
fn authentication_subject_goes_to_user_and_write_subject_to_actor() {
    // Arrange
    let robot = Some(principal(PrincipalKind::Robot));
    let cases = [
        event(AuditAction::Authenticate, failure(), None),
        event(
            AuditAction::Publish(coord("serde", "1.0.0")),
            Outcome::Success,
            robot,
        ),
    ];

    // Act
    let actual = cases.map(|event| {
        let r = ocsf(&event, Uuid::nil(), 0);
        (r.user.map(|u| u.type_id), r.actor.map(|u| u.type_id))
    });

    // Assert
    assert_eq!(actual, [(Some(0), None), (None, Some(99))]);
}

#[test]
fn write_event_carries_resource_request_context_and_stamp() {
    // Arrange
    let event = event(
        AuditAction::Publish(coord("serde", "1.0.0")),
        Outcome::Success,
        Some(principal(PrincipalKind::Robot)),
    );

    // Act
    let r = ocsf(&event, Uuid::from_u128(5), 1234);

    // Assert
    assert_eq!(
        (
            r.event_id,
            r.time_ms,
            r.correlation_uid.as_str(),
            r.src_ip.as_deref(),
            r.user_agent.as_deref(),
            r.resource_uid.as_deref(),
            r.resource_type,
            r.repository.as_deref(),
            r.trace_id.as_deref(),
        ),
        (
            Uuid::from_u128(5),
            1234,
            "req-1",
            Some("10.0.4.17"),
            Some("cargo/1.90"),
            Some("pkg:cargo/serde@1.0.0"),
            Some("package"),
            Some("internal"),
            Some("4bf92f3577b34da6a3ce929d0e0e4736"),
        )
    );
}

#[test]
fn authentication_event_has_no_resource() {
    // Arrange
    let event = event(AuditAction::Authenticate, failure(), None);

    // Act
    let r = ocsf(&event, Uuid::nil(), 0);

    // Assert
    assert_eq!(
        (r.resource_uid, r.resource_type, r.repository, r.operation),
        (None, None, None, None)
    );
}
