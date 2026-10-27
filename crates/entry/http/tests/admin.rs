//! Пробы admin listener: живость не зависит от хранилищ, готовность их спрашивает.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use std::future::{Future, pending, ready};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use crane_app::registry::Registry;
use crane_domain::registry::Sha256;
use crane_http::admin_router;
use crane_memory::{FixedClock, MemoryAudit, MemoryBlobStore, MemoryCatalog, SequentialIdGen};
use crane_ports::registry::{BlobError, BlobStore, ByteStream, Digests, Staged};
use http_body_util::BodyExt;
use serde_json::Value;
use tokio::runtime::Builder;
use tower::ServiceExt;

/// Как ведёт себя blob-хранилище на вопросе пробы.
#[derive(Clone, Copy)]
enum Probe {
    Fails,
    Hangs,
}

/// Blob-хранилище, у которого `open` (вопрос пробы) отказывает или не отвечает.
struct ProbeBlobs(Probe);

impl BlobStore for ProbeBlobs {
    fn stage(&self, _body: ByteStream) -> impl Future<Output = Result<Staged, BlobError>> + Send {
        ready(Err(BlobError::NotFound(Sha256::from([0; 32]))))
    }

    fn commit(&self, _staged: Staged) -> impl Future<Output = Result<Digests, BlobError>> + Send {
        ready(Err(BlobError::NotFound(Sha256::from([0; 32]))))
    }

    fn open(&self, _sha256: &Sha256) -> impl Future<Output = Result<ByteStream, BlobError>> + Send {
        let probe = self.0;
        async move {
            match probe {
                Probe::Fails => Err(BlobError::Backend("s3://b?key=SECRETPW".into())),
                Probe::Hangs => pending().await,
            }
        }
    }
}

fn registry<B: BlobStore>(blobs: B) -> Arc<Registry<MemoryCatalog, B>> {
    Arc::new(Registry::new(
        MemoryCatalog::default(),
        blobs,
        Arc::new(FixedClock::new(SystemTime::UNIX_EPOCH)),
        Arc::new(SequentialIdGen::default()),
        Arc::new(MemoryAudit::default()),
    ))
}

fn get(router: &Router, uri: &str) -> (StatusCode, String, Value) {
    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
    runtime.block_on(async {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        let (parts, body) = response.into_parts();
        let bytes = body.collect().await.unwrap().to_bytes();
        let content_type = parts
            .headers
            .get("content-type")
            .map_or_else(String::new, |value| value.to_str().unwrap().to_owned());
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (parts.status, content_type, json)
    })
}

#[test]
fn ready_with_healthy_stores_answers_ok() {
    // Arrange
    let router = admin_router(registry(MemoryBlobStore::default()));

    // Act
    let (status, _, _) = get(&router, "/healthz/ready");

    // Assert
    assert_eq!(status, StatusCode::OK);
}

#[test]
fn ready_with_failing_store_is_problem_json_503_without_leaking_the_cause() {
    // Arrange
    let router = admin_router(registry(ProbeBlobs(Probe::Fails)));

    // Act
    let reply = get(&router, "/healthz/ready");

    // Assert
    let (status, content_type, body) = reply;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(content_type, "application/problem+json");
    assert_eq!(body["title"], "Service Unavailable");
    assert_eq!(body["detail"], "хранилище недоступно");
    assert!(!body.to_string().contains("SECRETPW"), "{body}");
}

/// Проба `crane healthcheck` ждёт 2 с: ответ «не готов» обязан прийти раньше.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Статус и время, потраченное на ответ.
fn timed_status(router: &Router, uri: &str) -> (StatusCode, Duration) {
    let started = Instant::now();
    let (status, _, _) = get(router, uri);
    (status, started.elapsed())
}

#[test]
fn ready_with_hanging_store_answers_503_before_probe_client_gives_up() {
    // Arrange
    let router = admin_router(registry(ProbeBlobs(Probe::Hangs)));

    // Act
    let (status, elapsed) = timed_status(&router, "/healthz/ready");

    // Assert
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(elapsed < PROBE_TIMEOUT, "ответ за {elapsed:?}");
}

#[test]
fn live_stays_ok_when_stores_are_down() {
    // Arrange
    let router = admin_router(registry(ProbeBlobs(Probe::Fails)));

    // Act
    let (status, _, _) = get(&router, "/healthz/live");

    // Assert
    assert_eq!(status, StatusCode::OK);
}
