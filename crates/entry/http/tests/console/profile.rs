//! `GET /me` и `GET /status`.

use axum::Router;
use axum::http::Method;
use axum::routing::get;
use crane_http::{NavLink, Section};
use serde_json::json;

use crate::fixture::{Console, PRINCIPAL};

const FINDINGS: NavLink = NavLink {
    rel: "crane:findings",
    href: "/api/v1/findings",
};

fn section(nav: &'static [NavLink]) -> Section {
    Section {
        router: Router::new().route("/findings", get(|| async { "ok" })),
        nav,
    }
}

#[test]
fn me_describes_the_static_token_principal() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get("/api/v1/me");

    // Assert
    let body = reply.json();
    assert_eq!(
        (
            &body["principal"],
            &body["roles"],
            &body["workspaces"],
            &body["sod"]
        ),
        (
            &json!({"id": PRINCIPAL, "kind": "robot", "display": PRINCIPAL}),
            &json!(["platform-admin"]),
            &json!([]),
            &json!({"mode": "warn", "conflict": false}),
        )
    );
    assert_eq!(reply.header("content-type"), "application/hal+json");
}

#[test]
fn me_links_cover_self_and_the_console_own_sections() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.get("/api/v1/me");

    // Assert
    assert_eq!(
        reply.json()["_links"],
        json!({
            "self": {"href": "/api/v1/me"},
            "crane:status": {"href": "/api/v1/status"},
            "crane:settings": {"href": "/api/v1/settings"},
        })
    );
}

#[test]
fn me_links_include_a_mounted_section_only_when_it_declares_navigation() {
    // Arrange
    let with_nav = Console::with_sections(vec![section(&[FINDINGS])]);
    let without_nav = Console::with_sections(vec![section(&[])]);

    // Act
    let links = [&with_nav, &without_nav]
        .map(|console| console.get("/api/v1/me").json()["_links"]["crane:findings"].clone());

    // Assert
    assert_eq!(links, [json!({"href": "/api/v1/findings"}), json!(null)]);
}

#[test]
fn an_unmounted_section_has_no_route_and_no_link() {
    // Arrange
    let console = Console::new();

    // Act
    let replies = ["/api/v1/findings", "/api/v1/me"].map(|uri| console.get(uri));

    // Assert
    assert_eq!(
        (
            replies[0].status.as_u16(),
            replies[1].json()["_links"]["crane:findings"].clone()
        ),
        (404, json!(null))
    );
}

#[test]
fn me_revalidates_with_etag() {
    // Arrange
    let console = Console::new();
    let tag = console.get("/api/v1/me").header("etag").to_owned();

    // Act
    let reply = console.authed(Method::GET, "/api/v1/me", &[("if-none-match", &tag)], "");

    // Assert
    assert_eq!((reply.status.as_u16(), reply.body.len()), (304, 0));
}

#[test]
fn me_etag_changes_when_the_navigation_changes() {
    // Arrange
    let plain = Console::new();
    let with_section = Console::with_sections(vec![section(&[FINDINGS])]);

    // Act
    let tags =
        [&plain, &with_section].map(|console| console.get("/api/v1/me").header("etag").to_owned());

    // Assert
    assert_ne!(tags[0], tags[1]);
}

#[test]
fn status_without_a_token_has_only_the_public_part() {
    // Arrange
    let console = Console::new();

    // Act
    let reply = console.call(Method::GET, "/api/v1/status", &[], "");

    // Assert
    assert_eq!(
        reply.json(),
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "mode": "normal",
            "phase": "serving",
            "_links": {"self": {"href": "/api/v1/status"}},
        })
    );
}

#[test]
fn status_with_a_token_tells_whether_a_restart_is_required() {
    // Arrange
    let console = Console::new();
    let before = console.get("/api/v1/status").json()["restart_required"].clone();
    console.create("blobs", "file:///data");

    // Act
    let after = console.get("/api/v1/status").json()["restart_required"].clone();

    // Assert
    assert_eq!((before, after), (json!(false), json!(true)));
}

#[test]
fn status_with_a_token_reports_an_unreachable_settings_store() {
    // Arrange
    let console = Console::new();
    console.store_down();

    // Act
    let reply = console.get("/api/v1/status");

    // Assert
    assert_eq!(
        (reply.status.as_u16(), reply.header("retry-after")),
        (503, "10")
    );
}

#[test]
fn status_public_part_survives_an_unreachable_settings_store() {
    // Arrange
    let console = Console::new();
    console.store_down();

    // Act
    let reply = console.call(Method::GET, "/api/v1/status", &[], "");

    // Assert
    assert_eq!(reply.status.as_u16(), 200);
}
