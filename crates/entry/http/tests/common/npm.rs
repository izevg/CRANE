//! Фикстуры npm: документы публикации, запросы с Bearer-токеном, разбор ошибок.

use axum::http::Method;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use sha2::{Digest, Sha512};

use super::{Api, NPM_REPO, Reply, TOKEN};

pub(crate) const TARBALL: &[u8] = b"\x1f\x8b fake npm tarball";

/// SRI-сумма sha512 для `integrity`.
pub(crate) fn integrity(bytes: &[u8]) -> String {
    format!("sha512-{}", STANDARD.encode(Sha512::digest(bytes)))
}

/// Имя файла tarball: базовое имя после `/` для scope.
pub(crate) fn file_of(name: &str, version: &str) -> String {
    let base = name.rsplit('/').next().unwrap_or(name);
    format!("{base}-{version}.tgz")
}

/// Документ `PUT` публикации, как его шлёт npm CLI.
pub(crate) fn publish_doc(name: &str, version: &str, tarball: &[u8]) -> Value {
    json!({
        "_id": name,
        "name": name,
        "dist-tags": {"latest": version},
        "versions": {version: {
            "name": name,
            "version": version,
            "_id": format!("{name}@{version}"),
            "dependencies": {"left": "^1.0.0"},
            "dist": {
                "integrity": integrity(tarball),
                "tarball": format!("http://localhost:4873/{name}/-/{}", file_of(name, version)),
            },
        }},
        "_attachments": {file_of(name, version): {
            "content_type": "application/octet-stream",
            "data": STANDARD.encode(tarball),
            "length": tarball.len(),
        }},
    })
}

pub(crate) fn bearer() -> String {
    format!("Bearer {TOKEN}")
}

impl Api {
    /// Запрос npm-клиента с верным Bearer-токеном.
    pub(crate) fn npm(&self, method: Method, uri: &str, body: Vec<u8>) -> Reply {
        self.with_headers(method, uri, &[("authorization", &bearer())], body)
    }

    pub(crate) fn npm_get(&self, uri: &str) -> Reply {
        self.npm(Method::GET, uri, Vec::new())
    }

    pub(crate) fn npm_put(&self, uri: &str, body: &Value) -> Reply {
        self.npm(Method::PUT, uri, serde_json::to_vec(body).unwrap())
    }

    /// Публикует версию в hosted-репозиторий `web`.
    pub(crate) fn npm_publish(&self, name: &str, version: &str, tarball: &[u8]) -> Reply {
        self.npm_put(
            &format!("/npm/{NPM_REPO}/{}", name.replace('/', "%2f")),
            &publish_doc(name, version, tarball),
        )
    }
}

impl Reply {
    /// Текст npm-ошибки без суффикса `(request_id …)`; суффикс обязателен и равен заголовку.
    pub(crate) fn npm_error(&self) -> String {
        let json = self.json();
        let text = json["error"].as_str().unwrap();
        let suffix = format!(" (request_id {})", self.header("x-request-id"));
        text.strip_suffix(&suffix).unwrap().to_owned()
    }
}
