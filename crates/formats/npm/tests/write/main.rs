#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

#[path = "../common/mod.rs"]
mod common;

mod deprecate;
mod names;
mod publish;
mod scoped;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crane_format_npm::{NpmPublish, Write, WriteError, classify};
use serde_json::{Value, json};

use crate::common::pkg;

const TARBALL: &[u8] = b"tarball-bytes";

fn integrity() -> String {
    format!("sha512-{} sha384-ignored", STANDARD.encode([1u8; 64]))
}

fn manifest() -> Value {
    json!({
        "name": "left-pad",
        "version": "1.0.0",
        "dependencies": {"a": "^1.0.0"},
        "_hasShrinkwrap": false,
        "_id": "left-pad@1.0.0",
        "_nodeVersion": "20",
        "deprecated": "no",
        "publishConfig": {"registry": "https://evil.example/"},
        "dist": {
            "integrity": integrity(),
            "shasum": "ab".repeat(20),
            "tarball": "http://localhost:4873/left-pad/-/left-pad-1.0.0.tgz",
        },
    })
}

fn body() -> Value {
    json!({
        "_id": "left-pad",
        "name": "left-pad",
        "dist-tags": {"latest": "1.0.0"},
        "versions": {"1.0.0": manifest()},
        "_attachments": {
            "left-pad-1.0.0.tgz": {
                "content_type": "application/octet-stream",
                "data": STANDARD.encode(TARBALL),
                "length": TARBALL.len(),
            }
        },
    })
}

fn with(change: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value = body();
    change(&mut value);
    serde_json::to_vec(&value).unwrap()
}

fn with_manifest(change: impl FnOnce(&mut Value)) -> Vec<u8> {
    with(|b| change(&mut b["versions"]["1.0.0"]))
}

fn publish_result(bytes: &[u8]) -> Result<NpmPublish, WriteError> {
    publish_for("left-pad", bytes)
}

fn publish_for(name: &str, bytes: &[u8]) -> Result<NpmPublish, WriteError> {
    match classify(&pkg(name), bytes, &[])? {
        Write::Publish(publish) => Ok(publish),
        other => Err(WriteError::Json(format!("ожидалась публикация: {other:?}"))),
    }
}
