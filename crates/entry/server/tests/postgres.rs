//! Cargo-сценарий по HTTP на настоящих хранилищах: `PgCatalog` и `ObjectBlobStore` на файловой системе.

#![allow(
    clippy::expect_used,
    reason = "в тестах падение expect - это падение теста"
)]

mod support;

use std::sync::Arc;
use std::time::SystemTime;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use crane_app::registry::Registry;
use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_http::{CargoSettings, StaticToken, TrustedProxies, api_router};
use crane_memory::{FixedClock, MemoryAudit, SequentialIdGen};
use crane_objstore::ObjectBlobStore;
use crane_pg::PgCatalog;
use crane_ports::registry::Catalog;
use crane_ports::system::IdGen;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use url::Url;

const TOKEN: &str = "test-token";
const REPO: &str = "internal";
const CRATE: &[u8] = b"\x1f\x8b not really a gzip, but cargo does not look inside";

/// Приложение целиком; временный каталог blob живёт, пока жив `Stack`.
struct Stack {
    router: Router,
    _blobs_dir: tempfile::TempDir,
}

impl Stack {
    async fn new(pool: PgPool) -> Self {
        let blobs_dir = tempfile::tempdir().expect("временный каталог");
        let url = Url::from_directory_path(blobs_dir.path()).expect("абсолютный путь");
        let ids = Arc::new(SequentialIdGen::default());
        let catalog = PgCatalog::new(pool);
        catalog
            .ensure_repository(Repository {
                id: RepositoryId::new(ids.generate()),
                format: Format::Cargo,
                kind: RepositoryKind::Hosted,
                name: REPO.to_owned(),
            })
            .await
            .expect("репозиторий создаётся");
        let audit = Arc::new(MemoryAudit::default());
        let registry = Registry::new(
            catalog,
            ObjectBlobStore::from_url(&url).expect("blob-хранилище"),
            Arc::new(FixedClock::new(SystemTime::UNIX_EPOCH)),
            ids,
            audit.clone(),
        );
        let router = api_router(
            Arc::new(registry),
            CargoSettings {
                public_url: "https://crane.example".to_owned(),
                max_publish_bytes: 4096,
            },
            StaticToken::new(
                TOKEN.as_bytes(),
                Principal {
                    id: "crane".to_owned(),
                    kind: PrincipalKind::Robot,
                },
            ),
            TrustedProxies::parse("").expect("пустой список прокси"),
            audit,
        );
        Self {
            router,
            _blobs_dir: blobs_dir,
        }
    }

    async fn send(&self, method: Method, path: &str, body: Vec<u8>) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method(method)
            .uri(format!("/cargo/{REPO}/{path}"))
            .header("authorization", TOKEN)
            .body(Body::from(body))
            .expect("запрос");
        let response = self.router.clone().oneshot(request).await.expect("ответ");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("тело")
            .to_bytes();
        (status, bytes.to_vec())
    }

    async fn publish(&self) -> StatusCode {
        self.send(Method::PUT, "api/v1/crates/new", publish_body())
            .await
            .0
    }

    /// Единственная строка индекса `demo-crate` как JSON.
    async fn index_line(&self) -> Value {
        let (_, body) = self
            .send(Method::GET, "index/de/mo/demo-crate", Vec::new())
            .await;
        let text = String::from_utf8(body).expect("индекс - UTF-8");
        serde_json::from_str(text.trim()).expect("строка индекса - JSON")
    }
}

/// Тело `PUT /api/v1/crates/new`: длина JSON, JSON, длина крейта, крейт.
fn publish_body() -> Vec<u8> {
    let meta = json!({
        "name": "demo-crate", "vers": "1.2.3", "deps": [], "features": {},
        "description": "тестовый крейт", "license": "MIT"
    })
    .to_string();
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(meta.len()).expect("длина").to_le_bytes());
    out.extend_from_slice(meta.as_bytes());
    out.extend_from_slice(&u32::try_from(CRATE.len()).expect("длина").to_le_bytes());
    out.extend_from_slice(CRATE);
    out
}

#[test]
fn published_version_appears_in_index_with_checksum_of_the_uploaded_crate() {
    support::run(async |pool| {
        // Arrange
        let stack = Stack::new(pool).await;

        // Act
        let status = stack.publish().await;

        // Assert
        assert_eq!(status, StatusCode::OK);
        let line = stack.index_line().await;
        assert_eq!(line["cksum"], format!("{:x}", Sha256::digest(CRATE)));
        assert_eq!(line["vers"], "1.2.3");
    });
}

#[test]
fn download_returns_exactly_the_published_bytes() {
    support::run(async |pool| {
        // Arrange
        let stack = Stack::new(pool).await;
        stack.publish().await;

        // Act
        let (status, body) = stack
            .send(
                Method::GET,
                "api/v1/crates/demo-crate/1.2.3/download",
                Vec::new(),
            )
            .await;

        // Assert
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, CRATE);
    });
}

#[test]
fn yank_is_visible_in_the_index() {
    support::run(async |pool| {
        // Arrange
        let stack = Stack::new(pool).await;
        stack.publish().await;

        // Act
        let (status, _) = stack
            .send(
                Method::DELETE,
                "api/v1/crates/demo-crate/1.2.3/yank",
                Vec::new(),
            )
            .await;

        // Assert
        assert_eq!(status, StatusCode::OK);
        assert_eq!(stack.index_line().await["yanked"], true);
    });
}
