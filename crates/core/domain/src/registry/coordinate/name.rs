use std::fmt;
use std::hash::{Hash, Hasher};

use super::Format;

const CARGO_MAX_LEN: usize = 64;

/// Ошибка разбора имени пакета.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("имя пустое")]
    Empty,
    #[error("имя длиннее допустимого: {len} > {max}")]
    TooLong { len: usize, max: usize },
    #[error("недопустимый символ {0:?}")]
    InvalidChar(char),
    #[error("имя не может начинаться с {0:?}")]
    InvalidStart(char),
    #[error("формат {0:?} пока не поддержан")]
    Unsupported(Format),
}

/// Имя пакета: равенство и хеш считаются по нормализованной форме,
/// а исходное написание сохраняется для отдачи клиентам.
#[derive(Debug, Clone)]
pub struct PackageName {
    normalized: String,
    written: String,
}

impl PackageName {
    pub fn parse(format: Format, raw: &str) -> Result<Self, NameError> {
        match format {
            Format::Cargo => Self::parse_cargo(raw),
            Format::Maven | Format::Npm | Format::PyPi | Format::NuGet => {
                Err(NameError::Unsupported(format))
            }
        }
    }

    /// Правила crates.io; зарезервированные имена — политика, а не грамматика.
    fn parse_cargo(raw: &str) -> Result<Self, NameError> {
        let Some(first) = raw.chars().next() else {
            return Err(NameError::Empty);
        };
        let len = raw.chars().count();
        if len > CARGO_MAX_LEN {
            return Err(NameError::TooLong {
                len,
                max: CARGO_MAX_LEN,
            });
        }
        if !first.is_ascii_alphabetic() {
            return Err(NameError::InvalidStart(first));
        }
        if let Some(bad) = raw
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
        {
            return Err(NameError::InvalidChar(bad));
        }
        // Как canon_crate_name в crates.io: `-` и `_` обозначают одно имя.
        let normalized = raw.to_ascii_lowercase().replace('-', "_");
        Ok(Self {
            normalized,
            written: raw.to_owned(),
        })
    }

    #[must_use]
    pub fn normalized(&self) -> &str {
        &self.normalized
    }

    #[must_use]
    pub fn as_written(&self) -> &str {
        &self.written
    }
}

impl PartialEq for PackageName {
    fn eq(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

impl Eq for PackageName {}

impl Hash for PackageName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.written)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    fn cargo(raw: &str) -> Result<PackageName, NameError> {
        PackageName::parse(Format::Cargo, raw)
    }

    fn hash_of(name: &PackageName) -> u64 {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn dash_underscore_and_case_variants_are_equal() {
        // Arrange
        let a = cargo("Foo-Bar").unwrap();
        let b = cargo("foo_bar").unwrap();
        let c = cargo("FOO_BAR").unwrap();

        // Act
        let equal = a == b && b == c;

        // Assert
        assert!(equal);
    }

    #[test]
    fn equal_names_have_equal_hash() {
        // Arrange
        let a = cargo("Foo-Bar").unwrap();
        let b = cargo("FOO_BAR").unwrap();

        // Act
        let (ha, hb) = (hash_of(&a), hash_of(&b));

        // Assert
        assert_eq!(ha, hb);
    }

    #[test]
    fn different_names_are_not_equal() {
        // Arrange
        let a = cargo("foo-bar").unwrap();
        let b = cargo("foo-baz").unwrap();

        // Act
        let equal = a == b;

        // Assert
        assert!(!equal);
    }

    #[test]
    fn normalized_form_is_lowercase_with_underscores() {
        // Arrange
        let name = cargo("Foo-Bar").unwrap();

        // Act
        let normalized = name.normalized();

        // Assert
        assert_eq!(normalized, "foo_bar");
    }

    #[test]
    fn written_form_is_preserved_as_is() {
        // Arrange
        let name = cargo("Foo-Bar").unwrap();

        // Act
        let (written, shown) = (name.as_written().to_owned(), name.to_string());

        // Assert
        assert_eq!((written.as_str(), shown.as_str()), ("Foo-Bar", "Foo-Bar"));
    }

    #[test]
    fn name_of_max_length_is_accepted() {
        // Arrange
        let raw = "a".repeat(64);

        // Act
        let result = cargo(&raw);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn name_over_max_length_is_rejected() {
        // Arrange
        let raw = "a".repeat(65);

        // Act
        let result = cargo(&raw);

        // Assert
        assert_eq!(result.unwrap_err(), NameError::TooLong { len: 65, max: 64 });
    }

    #[test]
    fn empty_name_is_rejected() {
        // Arrange
        let raw = "";

        // Act
        let result = cargo(raw);

        // Assert
        assert_eq!(result.unwrap_err(), NameError::Empty);
    }

    #[test]
    fn name_starting_with_non_letter_is_rejected() {
        // Arrange
        let raws = ["1abc", "-abc", "_abc"];

        // Act
        let errors: Vec<_> = raws.iter().map(|r| cargo(r).unwrap_err()).collect();

        // Assert
        assert_eq!(
            errors,
            [
                NameError::InvalidStart('1'),
                NameError::InvalidStart('-'),
                NameError::InvalidStart('_'),
            ]
        );
    }

    #[test]
    fn name_with_forbidden_character_is_rejected() {
        // Arrange
        let raws = ["abé", "a b", "a.b"];

        // Act
        let errors: Vec<_> = raws.iter().map(|r| cargo(r).unwrap_err()).collect();

        // Assert
        assert_eq!(
            errors,
            [
                NameError::InvalidChar('é'),
                NameError::InvalidChar(' '),
                NameError::InvalidChar('.'),
            ]
        );
    }

    #[test]
    fn unsupported_format_is_reported() {
        // Arrange
        let format = Format::Npm;

        // Act
        let result = PackageName::parse(format, "left-pad");

        // Assert
        assert_eq!(result.unwrap_err(), NameError::Unsupported(Format::Npm));
    }

    #[test]
    fn different_names_have_different_hashes() {
        // Arrange
        let (a, b) = (cargo("foo").unwrap(), cargo("bar").unwrap());

        // Act
        let (hash_a, hash_b) = (hash_of(&a), hash_of(&b));

        // Assert
        assert_ne!(hash_a, hash_b);
    }
}
