//! Алгоритм паттернов на простой экосистеме; правила настоящих форматов проверяет
//! `crane-server` рядом с выбором реализации.

use super::*;
use crate::registry::{Version, VersionError};

/// Слова через `-`, регистр и `_` не значимы, имя не начинается с цифры и не длиннее `MAX_LEN`.
const MAX_LEN: usize = 20;

struct Dashed;

impl Ecosystem for Dashed {
    fn format(&self) -> Format {
        Format::Cargo
    }

    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        match raw.chars().next() {
            None => Err(NameError::Empty),
            Some(first) if first.is_ascii_digit() => Err(NameError::InvalidStart(first)),
            Some(_) if raw.len() > MAX_LEN => Err(NameError::TooLong {
                len: raw.len(),
                max: MAX_LEN,
            }),
            Some(_) => Ok(PackageName::from_parts(self.normalize(raw), raw.to_owned())),
        }
    }

    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::Cargo))
    }

    fn normalize(&self, raw: &str) -> String {
        raw.to_ascii_lowercase().replace('_', "-")
    }

    fn separator(&self) -> char {
        '-'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['-', '_']
    }
}

fn strings(patterns: &[NamespacePattern]) -> Vec<&str> {
    patterns.iter().map(NamespacePattern::as_str).collect()
}

#[test]
fn parse_normalizes_exact_and_prefix_patterns() {
    // Arrange
    let cases = ["Acme_*", "Acme_Pay", "acme-pay-*"];

    // Act
    let actual = cases.map(|raw| {
        NamespacePattern::parse(&Dashed, raw).map(|p| (p.as_str().to_owned(), p.is_prefix()))
    });

    // Assert
    let expected = [("acme-*", true), ("acme-pay", false), ("acme-pay-*", true)]
        .map(|(text, prefix)| Ok((text.to_owned(), prefix)));
    assert_eq!(actual, expected);
}

#[test]
fn parse_rejects_malformed_patterns() {
    // Arrange
    let cases = [
        "", "*", "a*b", "a**", "acme*", "-*", "acme--*", "1acme-*", "ac me", "1acme",
    ];

    // Act
    let actual = cases.map(|raw| NamespacePattern::parse(&Dashed, raw).unwrap_err());

    // Assert
    assert_eq!(
        actual,
        [
            PatternError::Empty,
            PatternError::EmptyBase,
            PatternError::Wildcard,
            PatternError::Wildcard,
            PatternError::Boundary,
            PatternError::EmptyBase,
            PatternError::Boundary,
            PatternError::Name(NameError::InvalidStart('1')),
            PatternError::Charset(' '),
            PatternError::Name(NameError::InvalidStart('1')),
        ]
    );
}

#[test]
fn covering_lists_patterns_from_most_specific_to_most_general() {
    // Arrange
    let name = Dashed.parse_name("acme-payments-core").unwrap();

    // Act
    let actual = NamespacePattern::covering(&Dashed, &name);

    // Assert
    assert_eq!(
        strings(&actual),
        ["acme-payments-core", "acme-payments-*", "acme-*"]
    );
}

#[test]
fn covering_does_not_cross_segment_boundaries() {
    // Arrange
    let name = Dashed.parse_name("acmecorp").unwrap();

    // Act
    let actual = NamespacePattern::covering(&Dashed, &name);

    // Assert
    assert_eq!(strings(&actual), ["acmecorp"]);
}

#[test]
fn name_is_covered_by_its_own_prefix_pattern() {
    // Arrange
    let name = Dashed.parse_name("Acme-Payments").unwrap();
    let namespace = NamespacePattern::parse(&Dashed, "acme-*").unwrap();

    // Act
    let covering = NamespacePattern::covering(&Dashed, &name);

    // Assert
    assert_eq!(covering.last(), Some(&namespace));
}

#[test]
fn prefix_base_is_validated_without_its_separator() {
    // Arrange
    let cases = ["a".repeat(MAX_LEN), "a".repeat(MAX_LEN + 1)].map(|base| format!("{base}-*"));

    // Act
    let actual = cases
        .each_ref()
        .map(|raw| NamespacePattern::parse(&Dashed, raw).map(|p| p.is_prefix()));

    // Assert
    assert_eq!(
        actual,
        [
            Ok(true),
            Err(PatternError::Name(NameError::TooLong {
                len: MAX_LEN + 1,
                max: MAX_LEN
            })),
        ]
    );
}
