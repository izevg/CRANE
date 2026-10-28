//! События аудита: что пишет слой auth и что пишут use cases.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use axum::http::Method;
use common::cargo::publish_body_with_registry;
use common::{Api, PRINCIPAL, REPO, TOKEN};
use crane_app::registry::Coordinate;
use crane_domain::access::{Principal, PrincipalKind, RequestContext, RequestId};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::{Ecosystem, Format};
use crane_format_cargo::CARGO;
use serde_json::json;

const CONFIG_URL: &str = "/cargo/internal/index/config.json";
const CRATE: &[u8] = b"bytes";
const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

#[test]
fn rejected_token_records_one_failed_authentication_with_request_context() {
    // Arrange
    let api = Api::with_proxies("127.0.0.1");
    let headers = [
        ("authorization", "SECRET-wrong"),
        ("user-agent", "cargo/1.90"),
        ("traceparent", TRACEPARENT),
        ("x-forwarded-for", "203.0.113.9"),
        ("x-request-id", "abc-1"),
    ];

    // Act
    let reply = api.get_from("127.0.0.1:40000", CONFIG_URL, &headers);

    // Assert
    assert_eq!(reply.status, 403);
    assert_eq!(
        api.audit_events(),
        [AuditEvent {
            actor: None,
            request: RequestContext {
                request_id: RequestId::parse("abc-1").unwrap(),
                trace_id: Some("4bf92f3577b34da6a3ce929d0e0e4736".to_owned()),
                client_ip: "203.0.113.9".parse().ok(),
                user_agent: Some("cargo/1.90".to_owned()),
            },
            action: AuditAction::Authenticate,
            outcome: Outcome::Failure {
                reason: "токен отвергнут".to_owned(),
                code: None
            },
        }]
    );
}

#[test]
fn missing_token_records_no_event() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.with_headers(Method::GET, CONFIG_URL, &[], Vec::new());

    // Assert
    assert_eq!(reply.status, 401);
    assert_eq!(api.audit_events(), []);
}

#[test]
fn accepted_token_records_no_authentication_event() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.get(CONFIG_URL);

    // Assert
    assert_eq!(reply.status, 200);
    assert_eq!(api.audit_events(), []);
}

#[test]
fn publish_records_success_attributed_to_the_configured_principal() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Assert
    assert_eq!(reply.status, 200);
    let events = api.audit_events();
    // Публикация нового имени даёт два события: Publish и захват имени Reserve.
    assert_eq!(events.len(), 2);
    assert_eq!(
        (
            events[0].actor.clone(),
            matches!(events[0].action, AuditAction::Publish(_)),
            events[0].outcome.clone(),
            events[0].request.request_id.as_str() == reply.header("x-request-id"),
        ),
        (
            Some(Principal {
                id: PRINCIPAL.to_owned(),
                kind: PrincipalKind::Robot
            }),
            true,
            Outcome::Success,
            true
        )
    );
}

#[test]
fn reads_add_no_events_to_the_journal() {
    // Arrange
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.0.0", CRATE);

    // Act
    let reply = api.index_text("demo-crate");

    // Assert
    assert_eq!(reply.status, 200);
    assert_eq!(
        api.audit_events().len(),
        2,
        "только события публикации: Publish и Reserve"
    );
}

#[test]
fn publish_with_foreign_dependency_registry_is_rejected_and_audited() {
    // Arrange
    let api = Api::new();
    let body = publish_body_with_registry(
        "demo-crate",
        "1.0.0",
        CRATE,
        &json!("sparse+https://evil.example/index/"),
    );
    let headers = [("authorization", TOKEN)];

    // Act
    let reply = api.with_headers(
        Method::PUT,
        &format!("/cargo/{REPO}/api/v1/crates/new"),
        &headers,
        body,
    );

    // Assert
    assert_eq!(
        (
            reply.status.as_u16(),
            reply.detail().contains("serde"),
            api.audit_events(),
            api.index_text("demo-crate").status.as_u16(),
        ),
        (
            400,
            true,
            vec![AuditEvent {
                actor: Some(Principal {
                    id: PRINCIPAL.to_owned(),
                    kind: PrincipalKind::Robot
                }),
                request: RequestContext {
                    request_id: RequestId::parse(reply.header("x-request-id")).unwrap(),
                    trace_id: None,
                    client_ip: None,
                    user_agent: None,
                },
                action: AuditAction::Publish(Coordinate {
                    format: Format::Cargo,
                    repository: REPO.to_owned(),
                    package: CARGO.parse_name("demo-crate").unwrap(),
                    version: CARGO.parse_version("1.0.0").unwrap(),
                }),
                outcome: Outcome::Failure {
                    code: Some("foreign-registry-dependency"),
                    reason: "сторонний реестр: зависимость \"serde\", \
                             registry \"sparse+https://evil.example/index/\""
                        .to_owned()
                },
            }],
            404,
        )
    );
}
