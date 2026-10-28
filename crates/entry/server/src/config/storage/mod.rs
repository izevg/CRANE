//! Хранилища: каталог в PostgreSQL (`Database`) и blob (`Blobs`: память, `file://`, `s3://`).
//! Каталог разбирается отдельно и только из слоя env + файл, поэтому пароль БД в `Config` не едет.

mod blobs;
mod database;
mod key;

use crane_domain::settings::{Kind, SettingDef};

pub(crate) use blobs::Blobs;
pub(crate) use database::Database;
pub(crate) use key::settings_key;

use super::source::setting;

pub(super) const GROUP: &str = "storage";
pub(super) const DATABASE_URL: &str = "CRANE_DATABASE_URL";
const DATABASE_URL_FILE: &str = "CRANE_DATABASE_URL_FILE";
pub(super) const BLOBS: &str = "CRANE_BLOBS";
pub(super) const SETTINGS_KEY: &str = "CRANE_SETTINGS_KEY";
const SETTINGS_KEY_FILE: &str = "CRANE_SETTINGS_KEY_FILE";
// Ключи S3 читает object_store из окружения сам; здесь только пара `*_FILE`, которой он не знает.
// В VARS не входят: AWS_* не ключи TOML.
const S3_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
pub(super) const S3_KEY_ID_FILE: &str = "AWS_ACCESS_KEY_ID_FILE";
const S3_SECRET: &str = "AWS_SECRET_ACCESS_KEY";
pub(super) const S3_SECRET_FILE: &str = "AWS_SECRET_ACCESS_KEY_FILE";

pub(super) const VARS: [&str; 5] = [
    DATABASE_URL,
    DATABASE_URL_FILE,
    BLOBS,
    SETTINGS_KEY,
    SETTINGS_KEY_FILE,
];

/// Каталог и ключ шифрования читаются до слоя БД и держат его самого: из консоли их не менять.
pub(super) const SETTINGS: [SettingDef; 3] = [
    SettingDef {
        console: false,
        ..setting(GROUP, "database_url", DATABASE_URL, Kind::Secret)
    },
    setting(GROUP, "blobs", BLOBS, Kind::Url),
    SettingDef {
        console: false,
        ..setting(GROUP, "settings_key", SETTINGS_KEY, Kind::Secret)
    },
];

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
