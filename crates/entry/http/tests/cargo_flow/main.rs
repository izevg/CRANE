//! Полный cargo-сценарий через HTTP на in-memory адаптерах.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

#[path = "../common/mod.rs"]
mod common;
mod fallback;
mod owners;
mod publish;
mod read;
mod yank;

use crate::common::{Api, REPO};

const CRATE: &[u8] = b"\x1f\x8b fake crate bytes";

fn url(tail: &str) -> String {
    format!("/cargo/{REPO}/api/v1/crates/{tail}")
}

fn published() -> Api {
    let api = Api::new();
    api.publish(REPO, "demo-crate", "1.2.3", CRATE);
    api
}
