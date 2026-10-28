//! Proxy-движок, metadata: зоны свежести, адаптивный TTL, stale-if-error, negative-кэш.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod adaptive_ttl;
#[path = "../common/mod.rs"]
mod common;
mod degraded_input;
mod freshness_zones;
mod max_age;
mod negative_cache;
mod representations;
mod revalidation;
mod stale_if_error;

const DOC: &str = "se/rd/serde";
