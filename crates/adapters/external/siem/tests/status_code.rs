//! Код отказа по правилу попадает в `status_code` записи OCSF.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::access::{Principal, PrincipalKind, RequestContext, RequestId};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::{Coordinate, Format, OrderKeyBuilder, PackageName, Version};
use crane_siem::ocsf;
use uuid::Uuid;

fn event(outcome: Outcome) -> AuditEvent {
    AuditEvent {
        actor: Some(Principal {
            id: "crane".to_owned(),
            kind: PrincipalKind::Robot,
        }),
        request: RequestContext {
            request_id: RequestId::parse("req-1").unwrap(),
            trace_id: None,
            client_ip: None,
            user_agent: None,
        },
        action: AuditAction::Publish(Coordinate {
            format: Format::Cargo,
            repository: "internal".to_owned(),
            package: PackageName::from_parts("serde".to_owned(), "serde".to_owned()),
            version: Version::from_parts(
                "1.0.0".to_owned(),
                "1.0.0".to_owned(),
                OrderKeyBuilder::new().finish(),
                false,
            ),
        }),
        outcome,
    }
}

#[test]
fn failure_code_goes_to_status_code_and_absent_code_stays_none() {
    // Arrange
    let coded = Outcome::Failure {
        reason: "причина".to_owned(),
        code: Some("some-rule"),
    };
    let uncoded = Outcome::Failure {
        reason: "причина".to_owned(),
        code: None,
    };
    let cases = [coded, uncoded, Outcome::Success];

    // Act
    let statuses = cases.map(|outcome| ocsf(&event(outcome), Uuid::nil(), 0).status_code);

    // Assert
    assert_eq!(statuses, [Some("some-rule".to_owned()), None, None]);
}
