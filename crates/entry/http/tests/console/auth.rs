//! Вход в консоль: Bearer против `StaticToken`, отказы в журнале, публичная часть статуса.

use axum::Router;
use axum::extract::Extension;
use axum::http::Method;
use axum::routing::get;
use crane_domain::access::{Caller, RequestContext, RequestId};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_http::Section;

use crate::common::TOKEN;
use crate::fixture::{Console, PRINCIPAL};

/// Раздел, который отдаёт то, что слой аутентификации положил в расширения запроса.
fn whoami() -> Section {
    async fn who(Extension(caller): Extension<Caller>) -> String {
        format!(
            "{}|{}",
            caller.principal.id,
            caller.request.request_id.as_str()
        )
    }
    Section {
        router: Router::new().route("/whoami", get(who)),
        nav: &[],
    }
}

fn failed_authentication(request_id: &str) -> AuditEvent {
    AuditEvent {
        actor: None,
        request: RequestContext {
            request_id: RequestId::parse(request_id).unwrap(),
            trace_id: None,
            client_ip: None,
            user_agent: None,
        },
        action: AuditAction::Authenticate,
        outcome: Outcome::Failure {
            reason: "токен отвергнут".to_owned(),
            code: None,
        },
    }
}

#[test]
fn request_without_authorization_is_challenged_and_audited() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.call(Method::GET, "/api/v1/me", &[], "");

    // Assert
    assert_eq!(
        (reply.status.as_u16(), reply.header("www-authenticate")),
        (401, "Bearer")
    );
    assert_eq!(
        console.audit_events(),
        [failed_authentication(reply.header("x-request-id"))]
    );
}

#[test]
fn wrong_token_is_challenged_and_audited_without_echoing_it() {
    // Arrange
    let console = Console::new();
    let headers = [("authorization", "Bearer not-the-token")];

    // Act
    let reply = console.call(Method::GET, "/api/v1/me", &headers, "");

    // Assert
    assert_eq!(
        (reply.status.as_u16(), reply.header("www-authenticate")),
        (401, "Bearer")
    );
    assert_eq!(
        console.audit_events(),
        [failed_authentication(reply.header("x-request-id"))]
    );
    assert!(!reply.text().contains("not-the-token"));
}

#[test]
fn other_schemes_and_a_bare_token_are_not_accepted() {
    // Arrange
    let console = Console::new();
    let bare = TOKEN.to_owned();
    let basic = format!("Basic {TOKEN}");
    let cases = [bare, basic];

    // Act
    let statuses = cases.map(|value| {
        console
            .call(Method::GET, "/api/v1/me", &[("authorization", &value)], "")
            .status
            .as_u16()
    });

    // Assert
    assert_eq!(statuses, [401, 401]);
}

#[test]
fn right_token_passes_caller_with_principal_and_request_context_to_the_section() {
    // Arrange
    let console = Console::with_sections(vec![whoami()]);

    // Act
    let reply = console.get("/api/v1/whoami");

    // Assert
    assert_eq!(
        reply.text(),
        format!("{PRINCIPAL}|{}", reply.header("x-request-id"))
    );
}

#[test]
fn right_token_leaves_no_authentication_event() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get("/api/v1/me");

    // Assert
    assert_eq!(reply.status.as_u16(), 200);
    assert_eq!(console.audit_events(), []);
}

#[test]
fn unknown_path_is_unauthorized_without_a_token_and_not_found_with_one() {
    // Arrange
    let console = Console::new();

    let bearer = format!("Bearer {TOKEN}");
    let cases: [&[(&str, &str)]; 2] = [&[], &[("authorization", &bearer)]];

    // Act
    let statuses = cases.map(|headers| {
        console
            .call(Method::GET, "/api/v1/nothing", headers, "")
            .status
            .as_u16()
    });

    // Assert
    assert_eq!(statuses, [401, 404]);
}

#[test]
fn status_is_public_but_a_wrong_token_on_it_is_still_refused() {
    // Arrange
    let console = Console::new();
    let cases: [&[(&str, &str)]; 2] = [&[], &[("authorization", "Bearer nope")]];

    // Act
    let statuses = cases.map(|headers| {
        console
            .call(Method::GET, "/api/v1/status", headers, "")
            .status
            .as_u16()
    });

    // Assert
    assert_eq!(statuses, [200, 401]);
}

#[test]
fn only_reading_status_is_public() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.call(Method::POST, "/api/v1/status", &[], "");

    // Assert
    assert_eq!(reply.status.as_u16(), 401);
}
