//! Публикация: атомарность, неизменяемость версий, проверка заявленного sha256.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

#[path = "../common/mod.rs"]
mod common;
mod declared_checksums;
mod immutability;
mod released_record;
mod storage_failures;
mod target_repository;

use crane_app::registry::Registry;
use crane_domain::registry::{Format, PackageVersion};
use crane_ports::registry::{BlobStore, Catalog};
use futures_executor::block_on;

use crate::common::{HOSTED, caller, name};

const HELLO: [&[u8]; 2] = [b"hel", b"lo"];

/// Наблюдение для Assert: версии пакета `foo`, видимые клиентам.
fn listed_foo<C: Catalog, B: BlobStore>(registry: &Registry<C, B>) -> Vec<PackageVersion> {
    block_on(registry.versions(&caller(), Format::Cargo, HOSTED, &name("foo"))).unwrap()
}
