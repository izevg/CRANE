//! Hosted-репозиторий `PyPI` через HTTP: загрузка, Simple API, файлы, yank, защита имён.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod auth;
#[path = "../common/mod.rs"]
mod common;
mod fixtures;
mod limits;
mod namespaces;
mod read;
mod upload;
mod yank;
