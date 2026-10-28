//! Алгоритм паттернов на простой экосистеме; правила настоящих форматов проверяет
//! `crane-server` рядом с выбором реализации.

use super::*;
use crate::registry::{Format, Version, VersionError, hosted_name_allowed};

const MAX_LEN: usize = 20;

/// Правила тестовой экосистемы: форма, разделитель и допустимые написания перед `*`.
/// Имя не начинается с цифры и не длиннее `MAX_LEN`.
struct Rules {
    shape: NamespaceShape,
    separator: char,
    accepted: &'static [char],
}

/// Слова через `-`, регистр и `_` не значимы.
const DASHED: Rules = Rules {
    shape: NamespaceShape::Delimited,
    separator: '-',
    accepted: &['-', '_'],
};
const SCOPED: Rules = Rules {
    shape: NamespaceShape::Scoped,
    separator: '/',
    accepted: &['/'],
};
const GROUPED: Rules = Rules {
    shape: NamespaceShape::Grouped,
    separator: '.',
    accepted: &['.'],
};

impl Ecosystem for Rules {
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
        match self.shape {
            NamespaceShape::Delimited => raw.to_ascii_lowercase().replace('_', "-"),
            NamespaceShape::Scoped | NamespaceShape::Grouped => raw.to_owned(),
        }
    }

    fn separator(&self) -> char {
        self.separator
    }

    fn accepted_separators(&self) -> &'static [char] {
        self.accepted
    }

    fn namespace_shape(&self) -> NamespaceShape {
        self.shape
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
        NamespacePattern::parse(&DASHED, raw).map(|p| (p.as_str().to_owned(), p.is_prefix()))
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
    let actual = cases.map(|raw| NamespacePattern::parse(&DASHED, raw).unwrap_err());

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
    let name = DASHED.parse_name("acme-payments-core").unwrap();

    // Act
    let actual = NamespacePattern::covering(&DASHED, &name);

    // Assert
    assert_eq!(
        strings(&actual),
        ["acme-payments-core", "acme-payments-*", "acme-*"]
    );
}

#[test]
fn covering_does_not_cross_segment_boundaries() {
    // Arrange
    let name = DASHED.parse_name("acmecorp").unwrap();

    // Act
    let actual = NamespacePattern::covering(&DASHED, &name);

    // Assert
    assert_eq!(strings(&actual), ["acmecorp"]);
}

#[test]
fn name_is_covered_by_its_own_prefix_pattern() {
    // Arrange
    let name = DASHED.parse_name("Acme-Payments").unwrap();
    let namespace = NamespacePattern::parse(&DASHED, "acme-*").unwrap();

    // Act
    let covering = NamespacePattern::covering(&DASHED, &name);

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
        .map(|raw| NamespacePattern::parse(&DASHED, raw).map(|p| p.is_prefix()));

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

fn parse_all(eco: &Rules, raws: &[&str]) -> Vec<Result<String, PatternError>> {
    raws.iter()
        .map(|raw| NamespacePattern::parse(eco, raw).map(|p| p.as_str().to_owned()))
        .collect()
}

fn covering_of_name(eco: &Rules, normalized: &str) -> Vec<String> {
    let name = PackageName::from_parts(normalized.to_owned(), normalized.to_owned());
    NamespacePattern::covering(eco, &name)
        .iter()
        .map(|p| p.as_str().to_owned())
        .collect()
}

#[test]
fn scoped_shape_accepts_only_a_scope_as_prefix() {
    // Arrange
    let raws = ["@acme/*", "@acme/ui", "acme/*", "@/*", "@a/b/*", "@acme*"];

    // Act
    let actual = parse_all(&SCOPED, &raws);

    // Assert
    assert_eq!(
        actual,
        [
            Ok("@acme/*".to_owned()),
            Ok("@acme/ui".to_owned()),
            Err(PatternError::Scope),
            Err(PatternError::EmptyBase),
            Err(PatternError::Scope),
            Err(PatternError::Scope),
        ]
    );
}

#[test]
fn scoped_shape_is_covered_by_its_scope_only() {
    // Arrange
    let names = ["@acme/ui", "left-pad", "@/x"];

    // Act
    let actual = names.map(|name| covering_of_name(&SCOPED, name));

    // Assert
    assert_eq!(
        actual,
        [
            vec!["@acme/ui".to_owned(), "@acme/*".to_owned()],
            vec!["left-pad".to_owned()],
            vec!["@/x".to_owned()],
        ]
    );
}

#[test]
fn grouped_shape_takes_a_group_base_and_rejects_colon_in_prefix() {
    // Arrange
    let raws = [
        "com.acme.*",
        "com.acme:core",
        "com.acme:*",
        "com.acme*",
        ".*",
    ];

    // Act
    let actual = parse_all(&GROUPED, &raws);

    // Assert
    assert_eq!(
        actual,
        [
            Ok("com.acme.*".to_owned()),
            Ok("com.acme:core".to_owned()),
            Err(PatternError::GroupId),
            Err(PatternError::Boundary),
            Err(PatternError::EmptyBase),
        ]
    );
}

#[test]
fn grouped_shape_is_covered_by_group_prefixes_and_only_exactly_without_a_group() {
    // Arrange
    let names = ["com.acme.b:c", "com.acmecorp:x", "plain"];

    // Act
    let actual = names.map(|name| covering_of_name(&GROUPED, name));

    // Assert
    assert_eq!(
        actual,
        [
            vec![
                "com.acme.b:c".to_owned(),
                "com.acme.b.*".to_owned(),
                "com.acme.*".to_owned(),
                "com.*".to_owned()
            ],
            vec![
                "com.acmecorp:x".to_owned(),
                "com.acmecorp.*".to_owned(),
                "com.*".to_owned()
            ],
            vec!["plain".to_owned()],
        ]
    );
}

#[test]
fn only_scoped_shape_requires_a_scope_for_hosted_names() {
    // Arrange
    let cases = [
        (&DASHED, "left-pad"),
        (&SCOPED, "left-pad"),
        (&SCOPED, "@acme/ui"),
        (&GROUPED, "com.acme:core"),
    ];

    // Act
    let actual = cases.map(|(eco, name)| {
        hosted_name_allowed(
            eco,
            &PackageName::from_parts(name.to_owned(), name.to_owned()),
        )
    });

    // Assert
    assert_eq!(actual, [true, false, true, true]);
}
