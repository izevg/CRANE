use thiserror::Error;

use crate::registry::RepositoryId;

/// Предел длины пути: разумный потолок для URL пакета и защита ключа кэша от раздувания.
const PATH_MAX_LEN: usize = 1024;

/// Нормализованный относительный путь у upstream: склеивается с базой источника и не может сменить хост.
///
/// Регистр не меняется: пути upstream регистрозависимы, а нормализация имён — дело формата.
/// `@` разрешён (npm `@scope/name`). Сменить хост он не может: путь не начинается с `/` и не содержит `:`,
/// а база всегда заканчивается `/` (см. `UpstreamBase`), поэтому `@` попадает в путь, а не в authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpstreamPath(String);

/// Ключ записи proxy-кэша: репозиторий и путь у upstream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProxyKey {
    pub repository: RepositoryId,
    pub path: UpstreamPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathError {
    #[error("путь пуст")]
    Empty,
    #[error("путь длиннее {PATH_MAX_LEN} байт")]
    TooLong,
    #[error("путь не должен начинаться с «/»")]
    Absolute,
    #[error("в пути недопустим байт 0x{0:02x}: разрешён только видимый ASCII")]
    InvalidByte(u8),
    #[error("в пути недопустим символ «{0}»")]
    ForbiddenChar(char),
    #[error("пустой сегмент пути (в том числе хвостовой «/»): ключи кэша должны быть каноничны")]
    EmptySegment,
    #[error("сегмент «.» или «..» (в том числе в виде %2e) недопустим")]
    DotSegment,
}

impl UpstreamPath {
    /// Разбирает путь по строгим правилам; любое сомнение — отказ, потому что путь уходит во внешнюю сеть.
    pub fn parse(raw: &str) -> Result<Self, PathError> {
        if raw.is_empty() {
            return Err(PathError::Empty);
        }
        if raw.len() > PATH_MAX_LEN {
            return Err(PathError::TooLong);
        }
        if raw.starts_with('/') {
            return Err(PathError::Absolute);
        }
        for byte in raw.bytes() {
            if !(0x21..=0x7E).contains(&byte) {
                return Err(PathError::InvalidByte(byte));
            }
        }
        // `:` закрывает схему и порт, `?`/`#` отрезают хвост, `\` часть клиентов трактует как `/`.
        if let Some(forbidden) = raw.chars().find(|c| matches!(c, '\\' | '?' | '#' | ':')) {
            return Err(PathError::ForbiddenChar(forbidden));
        }
        for segment in raw.split('/') {
            if segment.is_empty() {
                return Err(PathError::EmptySegment);
            }
            if is_dot_segment(segment) {
                return Err(PathError::DotSegment);
            }
        }
        Ok(Self(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Сегмент, который upstream или промежуточный прокси сведёт к `.`/`..` после percent-декодирования.
fn is_dot_segment(segment: &str) -> bool {
    let decoded = segment.to_ascii_lowercase().replace("%2e", ".");
    decoded == "." || decoded == ".."
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_canonical_relative_paths() {
        // Arrange
        let cases = [
            "config.json",
            "se/rd/serde",
            "3/s/syn",
            "crates/serde/serde-1.0.0.crate",
            "@babel/core",
            "@babel%2fcore",
            "a..b",
            "a/.hidden/b",
            "..a",
            "%2e%2ea",
        ];

        // Act
        let parsed = cases.map(|raw| UpstreamPath::parse(raw).map(|p| p.as_str().to_owned()));

        // Assert
        assert_eq!(parsed, cases.map(|raw| Ok(raw.to_owned())));
    }

    #[test]
    fn parse_rejects_paths_that_could_escape_or_be_ambiguous() {
        // Arrange
        let too_long = "a".repeat(PATH_MAX_LEN + 1);
        let cases = [
            ("", PathError::Empty),
            (too_long.as_str(), PathError::TooLong),
            ("/a", PathError::Absolute),
            ("a b", PathError::InvalidByte(b' ')),
            ("a\tb", PathError::InvalidByte(b'\t')),
            ("é", PathError::InvalidByte(0xC3)),
            ("a\u{7f}", PathError::InvalidByte(0x7F)),
            ("http://x", PathError::ForbiddenChar(':')),
            ("a?b", PathError::ForbiddenChar('?')),
            ("a#b", PathError::ForbiddenChar('#')),
            ("a\\b", PathError::ForbiddenChar('\\')),
            ("a//b", PathError::EmptySegment),
            ("a/", PathError::EmptySegment),
            ("a/../b", PathError::DotSegment),
            ("a/./b", PathError::DotSegment),
            ("..", PathError::DotSegment),
            ("a/%2e%2E/b", PathError::DotSegment),
            ("a/.%2e/b", PathError::DotSegment),
            ("a/%2e./b", PathError::DotSegment),
            ("a/%2E/b", PathError::DotSegment),
        ];

        // Act
        let errors = cases
            .clone()
            .map(|(raw, _)| UpstreamPath::parse(raw).unwrap_err());

        // Assert
        assert_eq!(errors, cases.map(|(_, expected)| expected));
    }

    #[test]
    fn parse_accepts_path_of_exactly_max_length() {
        // Arrange
        let raw = "a".repeat(PATH_MAX_LEN);

        // Act
        let parsed = UpstreamPath::parse(&raw);

        // Assert
        assert_eq!(parsed.unwrap().as_str(), raw);
    }

    #[test]
    fn parse_keeps_case() {
        // Arrange
        let raw = "Foo/BAR";

        // Act
        let parsed = UpstreamPath::parse(raw).unwrap();

        // Assert
        assert_eq!(parsed.as_str(), "Foo/BAR");
    }
}
