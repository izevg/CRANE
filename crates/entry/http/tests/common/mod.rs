//! Фикстура: `api_router` на in-memory адаптерах с известным токеном и hosted-репозиторием `internal`.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "dead_code: фикстуры общие для тестовых бинарей; unwrap в тестах = падение теста"
)]

use std::sync::Arc;
use std::time::SystemTime;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use bytes::Bytes;
use crane_app::registry::Registry;
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_http::{CargoSettings, StaticToken, admin_router, api_router};
use crane_memory::{FixedClock, MemoryBlobStore, MemoryCatalog, SequentialIdGen};
use crane_ports::registry::Catalog;
use crane_ports::system::IdGen;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::runtime::{Builder, Runtime};
use tower::ServiceExt;

pub(crate) const TOKEN: &str = "s3cret-token";
pub(crate) const PUBLIC_URL: &str = "https://crane.example";
pub(crate) const MAX_PUBLISH_BYTES: usize = 4096;
pub(crate) const REPO: &str = "internal";
const PROXY_REPO: &str = "mirror";

pub(crate) struct Reply {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
}

impl Reply {
    pub(crate) fn text(&self) -> String {
        String::from_utf8(self.body.to_vec()).unwrap()
    }

    pub(crate) fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }

    pub(crate) fn header(&self, name: &str) -> &str {
        self.headers.get(name).unwrap().to_str().unwrap()
    }
}

pub(crate) struct Api {
    runtime: Runtime,
    router: Router,
    admin: Router,
}

impl Api {
    pub(crate) fn new() -> Self {
        let runtime = Builder::new_current_thread().enable_all().build().unwrap();
        let ids = Arc::new(SequentialIdGen::default());
        let catalog = MemoryCatalog::default();
        for (name, kind) in [
            (REPO, RepositoryKind::Hosted),
            (PROXY_REPO, RepositoryKind::Proxy),
        ] {
            runtime
                .block_on(catalog.ensure_repository(Repository {
                    id: RepositoryId::new(ids.generate()),
                    format: Format::Cargo,
                    kind,
                    name: name.to_owned(),
                }))
                .unwrap();
        }
        let registry = Registry::new(
            catalog,
            MemoryBlobStore::default(),
            Arc::new(FixedClock::new(SystemTime::UNIX_EPOCH)),
            ids,
        );
        let router = api_router(
            Arc::new(registry),
            CargoSettings {
                public_url: PUBLIC_URL.to_owned(),
                max_publish_bytes: MAX_PUBLISH_BYTES,
            },
            StaticToken::from_secret(TOKEN.as_bytes()),
        );
        Self {
            runtime,
            router,
            admin: admin_router(),
        }
    }

    fn call(&self, router: &Router, request: Request<Body>) -> Reply {
        self.runtime.block_on(async {
            let response = router.clone().oneshot(request).await.unwrap();
            let (parts, body) = response.into_parts();
            Reply {
                status: parts.status,
                headers: parts.headers,
                body: body.collect().await.unwrap().to_bytes(),
            }
        })
    }

    /// Запрос с верным токеном.
    pub(crate) fn authed(&self, method: Method, uri: &str, body: Vec<u8>) -> Reply {
        self.with_headers(
            method,
            uri,
            &[(header::AUTHORIZATION.as_str(), TOKEN)],
            body,
        )
    }

    pub(crate) fn get(&self, uri: &str) -> Reply {
        self.authed(Method::GET, uri, Vec::new())
    }

    pub(crate) fn with_headers(
        &self,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Reply {
        let mut builder = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        self.call(&self.router, builder.body(Body::from(body)).unwrap())
    }

    pub(crate) fn admin_get(&self, uri: &str) -> Reply {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        self.call(&self.admin, request)
    }

    pub(crate) fn publish(&self, repo: &str, name: &str, vers: &str, krate: &[u8]) -> Reply {
        self.authed(
            Method::PUT,
            &format!("/cargo/{repo}/api/v1/crates/new"),
            publish_body(name, vers, krate),
        )
    }

    pub(crate) fn index_text(&self, name: &str) -> Reply {
        self.get(&format!("/cargo/{REPO}/index/{}", index_path(name)))
    }
}

/// Раскладка индекса для коротких тестовых имён длиной >= 4.
pub(crate) fn index_path(name: &str) -> String {
    format!("{}/{}/{name}", &name[0..2], &name[2..4])
}

/// Тело `PUT /api/v1/crates/new`: длина JSON, JSON, длина крейта, крейт.
pub(crate) fn publish_body(name: &str, vers: &str, krate: &[u8]) -> Vec<u8> {
    let meta = json!({
        "name": name,
        "vers": vers,
        "deps": [{
            "name": "serde", "version_req": "^1.0", "features": ["derive"],
            "optional": false, "default_features": true, "target": null,
            "kind": "normal", "registry": null, "explicit_name_in_toml": null
        }],
        "features": { "default": ["std"], "std": [] },
        "description": "тестовый крейт",
        "license": "MIT"
    })
    .to_string();
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(meta.len()).unwrap().to_le_bytes());
    out.extend_from_slice(meta.as_bytes());
    out.extend_from_slice(&u32::try_from(krate.len()).unwrap().to_le_bytes());
    out.extend_from_slice(krate);
    out
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Единственная строка индекса как JSON.
pub(crate) fn single_index_line(reply: &Reply) -> Value {
    let text = reply.text();
    let mut lines = text.lines();
    let line = lines.next().unwrap();
    assert!(lines.next().is_none(), "ожидалась одна строка: {text}");
    serde_json::from_str(line).unwrap()
}
