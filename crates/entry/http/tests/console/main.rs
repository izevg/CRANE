//! Консоль Admin API: аутентификация, `/me`, `/status`, настройки.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

#[path = "../common/mod.rs"]
mod common;
mod fixture;

mod auth;
mod headers;
mod profile;
mod secrets;
mod settings;
mod writes;
