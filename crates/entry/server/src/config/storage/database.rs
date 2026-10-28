//! Параметры подключения к PostgreSQL.

use std::str::FromStr;

use sqlx::postgres::PgConnectOptions;

use std::io;
use std::path::Path;

use super::{DATABASE_URL, DATABASE_URL_FILE};
use crate::config::source::secret;
use crate::config::{ConfigError, MEMORY};
use crane_domain::settings::Secret;

/// Подстановка, когда у значения нет читаемой схемы: само значение может быть секретом.
pub(super) const NO_SCHEME: &str = "(нет схемы)";

/// Каталог: память или PostgreSQL. Строка с паролем разбирается сразу и тут же уничтожается,
/// дальше живут только разобранные параметры. Без `Debug`: пароль в журнал не попадёт.
pub(crate) enum Database {
    Memory,
    Postgres(Box<PgConnectOptions>),
}

impl Database {
    /// Только из слоя env + файл: каталог нужен, чтобы прочитать слой БД, и сам в нём лежать не может.
    /// `memory` пишется явно: молчаливый каталог в памяти в продакшене потерял бы данные при рестарте.
    pub(crate) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        let url = secret(var, read_file, DATABASE_URL, DATABASE_URL_FILE)?;
        if url.expose() == MEMORY {
            return Ok(Self::Memory);
        }
        Self::parse(&url)
    }

    /// Каталог переживает рестарт.
    pub(crate) const fn persistent(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub(super) fn parse(url: &Secret) -> Result<Self, ConfigError> {
        let value = url.expose();
        // `PgConnectOptions` не проверяет схему: `mysql://…` он принял бы как PostgreSQL.
        if !(value.starts_with("postgres://") || value.starts_with("postgresql://")) {
            return Err(unsupported(DATABASE_URL, value));
        }
        // Текст ошибки sqlx цитирует часть URL (например, значение `sslmode`), поэтому он не выводится.
        PgConnectOptions::from_str(value)
            .map(|options| Self::Postgres(Box::new(options)))
            .map_err(|_| ConfigError::Invalid {
                var: DATABASE_URL,
                reason: "не разобран как URL postgres://",
            })
    }
}

fn unsupported(var: &'static str, value: &str) -> ConfigError {
    ConfigError::Unsupported {
        var,
        scheme: scheme_of(value),
    }
}

/// Схема URL. В значении бывают логин и пароль, поэтому наружу уходит только она,
/// и только если похожа на схему.
pub(super) fn scheme_of(value: &str) -> String {
    match value.split_once("://") {
        Some((scheme, _))
            if !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) =>
        {
            scheme.to_owned()
        }
        _ => NO_SCHEME.to_owned(),
    }
}
