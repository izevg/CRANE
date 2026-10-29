//! Способы предъявить токен: какой заголовок и в какой схеме. Какие из них принимает формат,
//! решает `formats::credentials`.

use std::borrow::Cow;

use axum::http::{HeaderMap, header};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use crane_domain::registry::Format;

use crate::formats;

const BEARER: &[u8] = b"Bearer ";
const BASIC: &[u8] = b"Basic ";

/// Как клиент формата присылает секрет.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Credential {
    /// Значение `Authorization` как есть: cargo шлёт токен без схемы.
    Raw,
    /// `Authorization: Bearer <токен>`; схема без учёта регистра, один пробел.
    Bearer,
    /// `Authorization: Basic base64(пользователь:пароль)`: пользователь игнорируется, секрет — пароль.
    BasicPassword,
    /// Токен в отдельном заголовке, например `X-NuGet-ApiKey`.
    Header(&'static str),
}

impl Credential {
    fn header_name(self) -> &'static str {
        match self {
            Self::Raw | Self::Bearer | Self::BasicPassword => header::AUTHORIZATION.as_str(),
            Self::Header(name) => name,
        }
    }

    /// Секрет, если заголовок есть и схема подошла. Чужая схема или битое значение дают `None`,
    /// а не «токен как есть»: иначе чужую схему приняли бы за секрет.
    pub(crate) fn extract(self, headers: &HeaderMap) -> Option<Cow<'_, [u8]>> {
        let value = headers.get(self.header_name())?.as_bytes();
        match self {
            Self::Raw | Self::Header(_) => Some(Cow::Borrowed(value)),
            Self::Bearer => strip_scheme(value, BEARER).map(Cow::Borrowed),
            Self::BasicPassword => basic_password(strip_scheme(value, BASIC)?).map(Cow::Owned),
        }
    }
}

fn strip_scheme<'a>(value: &'a [u8], scheme: &[u8]) -> Option<&'a [u8]> {
    let (head, rest) = value.split_at_checked(scheme.len())?;
    head.eq_ignore_ascii_case(scheme).then_some(rest)
}

/// Пароль — всё после первого `:`: в пароле двоеточие допустимо, в имени пользователя нет (RFC 7617).
fn basic_password(encoded: &[u8]) -> Option<Vec<u8>> {
    let decoded = STANDARD.decode(encoded).ok()?;
    let colon = decoded.iter().position(|byte| *byte == b':')?;
    Some(decoded[colon + 1..].to_vec())
}

/// Есть ли хоть один заголовок, который формат принимает. Без него клиент ещё не пытался войти
/// (рукопожатие протокола, 401), с ним — предъявил что-то не то (403).
pub(crate) fn offered(format: Format, headers: &HeaderMap) -> bool {
    formats::credentials(format)
        .iter()
        .any(|credential| headers.contains_key(credential.header_name()))
}

/// Секрет по первому подошедшему способу формата.
pub(crate) fn presented(format: Format, headers: &HeaderMap) -> Option<Cow<'_, [u8]>> {
    formats::credentials(format)
        .iter()
        .find_map(|credential| credential.extract(headers))
}

#[cfg(test)]
#[path = "tests/credential.rs"]
mod tests;
