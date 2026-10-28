#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

#[path = "../common/mod.rs"]
mod common;

mod hosted;
mod tags;
mod upstream;

use crane_format_npm::TarballBase;
use serde_json::Value;

const BASE: TarballBase<'static> = TarballBase("https://h.example/npm/repo/");

fn parse(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
