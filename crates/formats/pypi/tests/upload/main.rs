//! Проверка загрузки (legacy upload API): поля формы, имя файла, путь, дайджест, метаданные.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

mod accepted;
#[path = "../common/mod.rs"]
mod common;
mod filename;
mod mismatch;
mod rules;

pub(crate) const WHEEL: &str = "Acme_Core-1.0.0-py3-none-any.whl";
pub(crate) const SDIST: &str = "acme_core-1.0.0.tar.gz";
pub(crate) const HEX: &str = "ab";

pub(crate) type Fields = Vec<(String, String)>;

/// Поля формы, как их шлёт twine для колеса.
pub(crate) fn wheel_form() -> Fields {
    [
        (":action", "file_upload"),
        ("protocol_version", "1"),
        ("name", "Acme_Core"),
        ("version", "1.0.0"),
        ("filetype", "bdist_wheel"),
        ("pyversion", "py3"),
        ("metadata_version", "2.1"),
    ]
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .to_vec()
}

pub(crate) fn with(mut fields: Fields, key: &str, value: &str) -> Fields {
    fields.retain(|(name, _)| name != key);
    fields.push((key.to_owned(), value.to_owned()));
    fields
}

pub(crate) fn without(mut fields: Fields, key: &str) -> Fields {
    fields.retain(|(name, _)| name != key);
    fields
}

pub(crate) fn sha256_hex() -> String {
    HEX.repeat(32)
}
