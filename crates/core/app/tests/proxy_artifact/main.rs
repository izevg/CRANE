//! Proxy-движок, артефакты: CAS, проверка sha256, перекачка потерянного blob, negative-кэш.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod cas_hits;
#[path = "../common/mod.rs"]
mod common;
mod digests;
mod negative_cache;
mod size_limit;
mod upstream_failures;

const CRATE: &str = "crates/serde/serde-1.0.0.crate";
