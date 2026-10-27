//! Чистое сопоставление `AuditEvent` с плоскими полями OCSF 1.3.

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::Coordinate;
use uuid::Uuid;

const SCHEMA_VERSION: &str = "1.3.0";
const PRODUCT_NAME: &str = "CRANE";
const CLASS_AUTHENTICATION: u32 = 3002;
const CLASS_API_ACTIVITY: u32 = 6003;
const CATEGORY_IAM: u32 = 3;
const CATEGORY_APPLICATION: u32 = 6;
const USER_TYPE_UNKNOWN: u32 = 0;
const USER_TYPE_USER: u32 = 1;
const USER_TYPE_OTHER: u32 = 99;

/// Субъект события: `actor.user.*` у 6003, `user.*` у 3002.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcsfUser {
    pub uid: Option<String>,
    pub type_id: u32,
    pub type_name: String,
}

/// Плоские поля одной записи. Необязательные значения — `None`: в журнал они не попадают.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcsfRecord {
    pub class_uid: u32,
    pub category_uid: u32,
    pub activity_id: u32,
    pub activity_name: &'static str,
    pub type_uid: u32,
    pub status_id: u32,
    pub status: &'static str,
    pub status_detail: Option<String>,
    pub severity_id: u32,
    pub event_id: Uuid,
    pub correlation_uid: String,
    pub schema_version: &'static str,
    pub product_name: &'static str,
    pub product_version: &'static str,
    pub time_ms: u64,
    pub actor: Option<OcsfUser>,
    pub user: Option<OcsfUser>,
    pub src_ip: Option<String>,
    pub user_agent: Option<String>,
    pub operation: Option<&'static str>,
    pub resource_uid: Option<String>,
    pub resource_type: Option<&'static str>,
    pub repository: Option<String>,
    pub trace_id: Option<String>,
}

/// Класс, категория, activity (id и имя) и операция API по действию.
struct Shape {
    class_uid: u32,
    category_uid: u32,
    activity_id: u32,
    activity_name: &'static str,
    operation: Option<&'static str>,
}

fn shape(action: &AuditAction) -> Shape {
    let api = |activity_id, activity_name, operation| Shape {
        class_uid: CLASS_API_ACTIVITY,
        category_uid: CATEGORY_APPLICATION,
        activity_id,
        activity_name,
        operation: Some(operation),
    };
    match action {
        AuditAction::Authenticate => Shape {
            class_uid: CLASS_AUTHENTICATION,
            category_uid: CATEGORY_IAM,
            activity_id: 1,
            activity_name: "Logon",
            operation: None,
        },
        AuditAction::Publish(_) => api(1, "Create", "publish"),
        AuditAction::Yank(_) => api(3, "Update", "yank"),
        AuditAction::Unyank(_) => api(3, "Update", "unyank"),
    }
}

fn coordinate(action: &AuditAction) -> Option<&Coordinate> {
    match action {
        AuditAction::Authenticate => None,
        AuditAction::Publish(c) | AuditAction::Yank(c) | AuditAction::Unyank(c) => Some(c),
    }
}

/// `+` в версии (метаданные сборки) кодируется: иначе purl неоднозначен.
fn purl(c: &Coordinate) -> String {
    format!(
        "pkg:cargo/{}@{}",
        c.package.as_written(),
        c.version.as_written().replace('+', "%2B")
    )
}

fn kind_name(kind: PrincipalKind) -> &'static str {
    match kind {
        PrincipalKind::Human => "User",
        PrincipalKind::Robot => "Robot",
        PrincipalKind::Workload => "Workload",
        PrincipalKind::Agent => "Agent",
    }
}

fn user(principal: Option<&Principal>) -> OcsfUser {
    match principal {
        None => OcsfUser {
            uid: None,
            type_id: USER_TYPE_UNKNOWN,
            type_name: "Unknown".to_owned(),
        },
        Some(p) => OcsfUser {
            uid: Some(p.id.clone()),
            type_id: if p.kind == PrincipalKind::Human {
                USER_TYPE_USER
            } else {
                USER_TYPE_OTHER
            },
            type_name: kind_name(p.kind).to_owned(),
        },
    }
}

/// `event_id` и время ставит приёмник.
#[must_use]
pub fn ocsf(event: &AuditEvent, event_id: Uuid, time_ms: u64) -> OcsfRecord {
    let shape = shape(&event.action);
    let subject = user(event.actor.as_ref());
    let is_auth = shape.class_uid == CLASS_AUTHENTICATION;
    let (status_id, status, severity_id, status_detail) = match &event.outcome {
        Outcome::Success => (1, "Success", 1, None),
        Outcome::Failure { reason } => (2, "Failure", 2, Some(reason.clone())),
    };
    let resource = coordinate(&event.action);
    OcsfRecord {
        class_uid: shape.class_uid,
        category_uid: shape.category_uid,
        activity_id: shape.activity_id,
        activity_name: shape.activity_name,
        type_uid: shape.class_uid * 100 + shape.activity_id,
        status_id,
        status,
        status_detail,
        severity_id,
        event_id,
        correlation_uid: event.request.request_id.as_str().to_owned(),
        schema_version: SCHEMA_VERSION,
        product_name: PRODUCT_NAME,
        product_version: env!("CARGO_PKG_VERSION"),
        time_ms,
        actor: (!is_auth).then(|| subject.clone()),
        user: is_auth.then_some(subject),
        src_ip: event.request.client_ip.map(|ip| ip.to_string()),
        user_agent: event.request.user_agent.clone(),
        operation: shape.operation,
        resource_uid: resource.map(purl),
        resource_type: resource.map(|_| "package"),
        repository: resource.map(|c| c.repository.clone()),
        trace_id: event.request.trace_id.clone(),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "в тестах падение expect — это падение теста"
)]
mod tests {
    use crane_domain::access::{RequestContext, RequestId};
    use crane_domain::registry::{Format, PackageName, Version};

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
            (AuditAction::Unyank(c()), failure(), robot),
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
            coord("serde", "1.0.0"),
            coord("Foo_Bar", "1.2.3-rc.1+build5"),
        ];

        // Act
        let actual = cases.map(|c| purl(&c));

        // Assert
        assert_eq!(
            actual,
            [
                "pkg:cargo/serde@1.0.0",
                "pkg:cargo/Foo_Bar@1.2.3-rc.1%2Bbuild5"
            ]
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
}
