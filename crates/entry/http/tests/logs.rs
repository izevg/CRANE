//! Журналы: в них попадает наблюдаемое и не попадает секретное.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use std::sync::Arc;
use std::time::SystemTime;

use axum::body::Body;
use axum::http::{Method, Request};
use common::{Api, REPO, Reply, SharedBuf, TOKEN, request};
use crane_memory::{FixedClock, MemoryAudit, SequentialIdGen};
use crane_siem::OcsfLogSink;
use tracing_subscriber::fmt;

const CRATE: &[u8] = b"bytes";
const SECRET_WRONG: &str = "SUPERSECRET-wrong";
/// `user:PASSWORD` в Basic.
const BASIC: &str = "Basic dXNlcjpQQVNTV09SRA==";

fn real_sink_api() -> Api {
    let sink = Arc::new(OcsfLogSink::new(
        Arc::new(FixedClock::new(SystemTime::UNIX_EPOCH)),
        Arc::new(SequentialIdGen::default()),
    ));
    Api::build("", sink, Arc::new(MemoryAudit::default()))
}

/// Перехваченный вывод JSON-журнала, как его пишет сервер (`flatten_event`).
fn capturing(buffer: &SharedBuf) -> impl tracing::Subscriber + Send + Sync + use<> {
    fmt()
        .json()
        .flatten_event(true)
        .with_writer(buffer.clone())
        .finish()
}

/// Какие из фрагментов встречаются в выводе, в исходном порядке.
fn contained<'a>(output: &str, needles: &[&'a str]) -> Vec<&'a str> {
    needles
        .iter()
        .copied()
        .filter(|needle| output.contains(needle))
        .collect()
}

/// Один вызов на всю последовательность запросов под перехватом журнала.
fn run_requests<const N: usize>(
    subscriber: impl tracing::Subscriber + Send + Sync,
    api: &Api,
    requests: [Request<Body>; N],
) -> [Reply; N] {
    tracing::subscriber::with_default(subscriber, || requests.map(|r| api.send(r)))
}

#[test]
fn logs_never_contain_presented_credentials_but_do_contain_audit_events() {
    // Arrange
    let buffer = SharedBuf::default();
    let api = real_sink_api();
    let publish_uri = format!("/cargo/{REPO}/api/v1/crates/new");
    let requests = [
        request(
            Method::GET,
            "/cargo/internal/index/config.json",
            &[("authorization", SECRET_WRONG)],
            Vec::new(),
            None,
        ),
        request(
            Method::GET,
            "/cargo/internal/index/config.json",
            &[("authorization", BASIC)],
            Vec::new(),
            None,
        ),
        request(
            Method::PUT,
            &publish_uri,
            &[("authorization", TOKEN)],
            common::publish_body("demo-crate", "1.0.0", CRATE),
            None,
        ),
    ];

    // Act
    let replies = run_requests(capturing(&buffer), &api, requests);

    // Assert
    assert_eq!(replies.map(|r| r.status.as_u16()), [403, 403, 200]);
    let output = buffer.text();
    let leaked = contained(
        &output,
        &["SUPERSECRET", "PASSWORD", "dXNlcjpQQVNTV09SRA==", TOKEN],
    );
    assert_eq!(leaked, Vec::<&str>::new());
    // Без этих двух строк пустой перехват тоже прошёл бы проверку на утечку.
    let seen = contained(
        &output,
        &[r#""class_uid":3002"#, r#""api.operation":"publish""#],
    );
    assert_eq!(
        seen,
        [r#""class_uid":3002"#, r#""api.operation":"publish""#]
    );
}

#[test]
fn access_line_has_path_without_query_and_the_authenticated_actor() {
    // Arrange
    let buffer = SharedBuf::default();
    let api = Api::new();
    let requests = [request(
        Method::GET,
        "/cargo/internal/index/config.json?token=LEAK",
        &[("authorization", TOKEN), ("user-agent", "cargo/1.90")],
        Vec::new(),
        None,
    )];

    // Act
    let replies = run_requests(capturing(&buffer), &api, requests);

    // Assert
    assert_eq!(replies[0].status, 200);
    let output = buffer.text();
    assert_eq!(contained(&output, &["LEAK", TOKEN]), Vec::<&str>::new());
    let expected = [
        r#""target":"crane::access""#,
        r#""path":"/cargo/internal/index/config.json""#,
        r#""actor":"crane""#,
        r#""status":200"#,
        r#""user_agent":"cargo/1.90""#,
    ];
    assert_eq!(contained(&output, &expected), expected);
}
