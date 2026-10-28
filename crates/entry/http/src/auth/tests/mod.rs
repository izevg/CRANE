use std::sync::Arc;

use axum::Extension;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use crane_domain::access::{PrincipalKind, RequestContext, RequestId};
use crane_memory::MemoryAudit;
use tokio::runtime::Builder;
use tower::ServiceExt;

use super::*;

const SECRET: &str = "s3cret";

fn token() -> StaticToken {
    StaticToken::new(
        SECRET.as_bytes(),
        Principal {
            id: "ci".to_owned(),
            kind: PrincipalKind::Robot,
        },
    )
}

/// Один запрос с заданными заголовками через слой аутентификации формата:
/// статус, `WWW-Authenticate` и число событий аудита.
fn attempt(format: Format, pairs: &[(&str, &[u8])]) -> (StatusCode, Option<String>, usize) {
    let audit = Arc::new(MemoryAudit::default());
    let auth = Arc::new(Auth {
        token: Arc::new(token()),
        audit: audit.clone(),
        format,
    });
    let context = RequestContext {
        request_id: RequestId::parse("req-1").unwrap(),
        trace_id: None,
        client_ip: None,
        user_agent: None,
    };
    let router = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(from_fn_with_state(auth, require_token))
        .layer(Extension(context));
    let mut request = Request::builder().uri("/");
    for (name, value) in pairs {
        request = request.header(*name, *value);
    }
    let response = Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(router.oneshot(request.body(Body::empty()).unwrap()))
        .unwrap();
    let challenge = response
        .headers()
        .get(header::WWW_AUTHENTICATE)
        .map(|value| value.to_str().unwrap().to_owned());
    (response.status(), challenge, audit.events().len())
}

#[test]
fn authenticate_accepts_only_the_exact_secret() {
    // Arrange
    let token = token();
    let presented: [&[u8]; 5] = [b"s3cret", b"s3cre", b"s3cret!", b"", b"other!"];

    // Act
    let accepted = presented.map(|value| token.authenticate(value).map(|p| p.id.as_str()));

    // Assert
    assert_eq!(accepted, [Some("ci"), None, None, None, None]);
}

#[test]
fn nuget_without_any_accepted_header_gets_a_challenge_and_no_audit_event() {
    // Arrange
    let pairs: [&[(&str, &[u8])]; 2] = [&[], &[("x-other", b"1")]];

    // Act
    let actual = pairs.map(|pairs| attempt(Format::NuGet, pairs));

    // Assert
    let challenge = Some("Basic realm=\"crane\"".to_owned());
    assert_eq!(
        actual,
        [
            (StatusCode::UNAUTHORIZED, challenge.clone(), 0),
            (StatusCode::UNAUTHORIZED, challenge, 0)
        ]
    );
}

#[test]
fn nuget_with_a_wrong_or_foreign_credential_is_forbidden_and_audited() {
    // Arrange
    let cases: [&[(&str, &[u8])]; 3] = [
        &[("x-nuget-apikey", b"wrong")],
        &[("authorization", b"Bearer s3cret")],
        &[("authorization", b"Basic !!!")],
    ];

    // Act
    let actual = cases.map(|pairs| attempt(Format::NuGet, pairs));

    // Assert
    assert_eq!(actual.to_vec(), vec![(StatusCode::FORBIDDEN, None, 1); 3]);
}

#[test]
fn nuget_accepts_the_api_key_header_and_basic_password() {
    // Arrange
    let basic = b"Basic dXNlcjpzM2NyZXQ=";
    let cases: [&[(&str, &[u8])]; 2] = [
        &[("x-nuget-apikey", SECRET.as_bytes())],
        &[("authorization", basic)],
    ];

    // Act
    let actual = cases.map(|pairs| attempt(Format::NuGet, pairs));

    // Assert
    assert_eq!(actual.to_vec(), vec![(StatusCode::OK, None, 0); 2]);
}
