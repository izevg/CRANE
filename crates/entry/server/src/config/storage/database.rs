//! Параметры подключения к PostgreSQL.

use std::str::FromStr;

use sqlx::postgres::PgConnectOptions;

use super::DATABASE_URL;
use crate::config::ConfigError;
use crate::config::source::Secret;

/// Подстановка, когда у значения нет читаемой схемы: само значение может быть секретом.
pub(super) const NO_SCHEME: &str = "(нет схемы)";

/// Параметры подключения к PostgreSQL. Строка с паролем разбирается сразу и тут же
/// уничтожается, дальше живут только разобранные параметры. Без `Debug`: пароль в журнал не попадёт.
pub(crate) struct Database(Box<PgConnectOptions>);

impl Database {
    pub(super) fn parse(url: &Secret) -> Result<Self, ConfigError> {
        let value = url.expose();
        // `PgConnectOptions` не проверяет схему: `mysql://…` он принял бы как PostgreSQL.
        if !(value.starts_with("postgres://") || value.starts_with("postgresql://")) {
            return Err(unsupported(DATABASE_URL, value));
        }
        // Текст ошибки sqlx цитирует часть URL (например, значение `sslmode`), поэтому он не выводится.
        PgConnectOptions::from_str(value)
            .map(|options| Self(Box::new(options)))
            .map_err(|_| ConfigError::Invalid {
                var: DATABASE_URL,
                reason: "не разобран как URL postgres://",
            })
    }

    pub(crate) fn options(&self) -> &PgConnectOptions {
        &self.0
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
