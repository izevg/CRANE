//! Недоступный каталог закрывает proxy: даже имя из кэша не отдаётся, пока нельзя проверить,
//! что оно не внутреннее, и в upstream тоже не ходим.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod common;

use axum::http::StatusCode;
use common::Api;
use common::cargo::sha256_hex;
use common::npm::{TARBALL, integrity};
use common::proxy::{NpmProxy, Proxy, Step};
use serde_json::json;

const RETRY_AFTER_SECONDS: &str = "10";
const CARGO_URI: &str = "/cargo/crates-io/index/se/rd/serde";
const CARGO_UPSTREAM_PATH: &str = "se/rd/serde";
const NPM_URI: &str = "/npm/npmjs/left-pad";
const NPM_UPSTREAM_PATH: &str = "left-pad";

fn serde_index() -> Vec<u8> {
    format!(
        r#"{{"name":"serde","vers":"1.0.0","deps":[],"cksum":"{}","features":{{}},"yanked":false}}{}"#,
        sha256_hex(b"crate"),
        "\n"
    )
    .into_bytes()
}

fn left_pad_packument() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "name": "left-pad",
        "dist-tags": {"latest": "1.0.0"},
        "versions": {"1.0.0": {
            "name": "left-pad",
            "version": "1.0.0",
            "dist": {
                "tarball": "https://registry.npmjs.org/left-pad/-/left-pad-1.0.0.tgz",
                "integrity": integrity(TARBALL),
            },
        }},
    }))
    .unwrap()
}

#[test]
fn cargo_index_from_warm_cache_is_unavailable_when_catalog_is_down() {
    // Arrange
    let proxy = Proxy::new();
    let api = Api::with_proxy(&proxy);
    proxy
        .upstream
        .will(CARGO_UPSTREAM_PATH, Step::Found(serde_index(), None));
    let warmed = api.get(CARGO_URI);
    api.catalog_down();

    // Act
    let reply = api.get(CARGO_URI);

    // Assert
    assert_eq!(warmed.status, StatusCode::OK, "прогрев кэша");
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.header("retry-after"), RETRY_AFTER_SECONDS);
    assert_eq!(
        proxy.upstream.call_count(),
        1,
        "после отказа в upstream не ходили"
    );
}

#[test]
fn npm_packument_from_warm_cache_is_unavailable_when_catalog_is_down() {
    // Arrange
    let proxy = NpmProxy::new();
    let api = Api::with_npm_proxy(&proxy);
    proxy
        .upstream
        .will(NPM_UPSTREAM_PATH, Step::Found(left_pad_packument(), None));
    let warmed = api.npm_get(NPM_URI);
    api.catalog_down();

    // Act
    let reply = api.npm_get(NPM_URI);

    // Assert
    assert_eq!(warmed.status, StatusCode::OK, "прогрев кэша");
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.header("retry-after"), RETRY_AFTER_SECONDS);
    assert_eq!(
        proxy.upstream.call_count(),
        1,
        "после отказа в upstream не ходили"
    );
}
