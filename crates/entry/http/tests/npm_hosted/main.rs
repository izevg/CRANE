//! Hosted-репозиторий npm через HTTP: публикация, чтение, устаревание, снятие, dist-tags.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod auth;
#[path = "../common/mod.rs"]
mod common;
mod deprecate_delete;
mod dist_tags;
mod publish;
mod read;

use crate::common::NPM_REPO;
use crate::common::npm::file_of;

fn packument_uri(name: &str) -> String {
    format!("/npm/{NPM_REPO}/{}", name.replace('/', "%2f"))
}

fn tarball_uri(name: &str, version: &str) -> String {
    format!("/npm/{NPM_REPO}/{name}/-/{}", file_of(name, version))
}
