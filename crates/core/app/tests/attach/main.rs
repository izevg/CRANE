//! Прикрепление файла к версии: создание первым файлом, повторы, гонка, отказы и аудит.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod audit;
#[path = "../common/mod.rs"]
mod common;
mod conflicts;
mod create;
mod rules;

use crane_app::registry::Registry;
use crane_domain::registry::Format;
use crane_ports::registry::{BlobStore, Catalog};
use futures_executor::block_on;

use crate::common::{HOSTED, caller, name};

const HELLO: [&[u8]; 1] = [b"hello"];
const FIRST: &str = "foo-1.0.0.crate";
const SECOND: &str = "foo-1.0.0.whl";

/// Наблюдение для Assert: пути файлов `foo 1.0.0` в порядке каталога.
fn paths_of_foo<C: Catalog, B: BlobStore>(registry: &Registry<C, B>) -> Vec<String> {
    block_on(registry.versions(&caller(), Format::Cargo, HOSTED, &name("foo")))
        .unwrap()
        .into_iter()
        .flat_map(|version| version.assets)
        .map(|asset| asset.path)
        .collect()
}
