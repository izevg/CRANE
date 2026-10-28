use crane_domain::registry::{
    Format, NameError, NamespacePattern, PackageName, PatternError, hosted_name_allowed,
};

use super::ecosystem;

/// Паттерны, покрывающие уже нормализованное имя (имена в таблицах заданы нормализованными).
fn covering(format: Format, normalized: &str) -> Vec<NamespacePattern> {
    let name = PackageName::from_parts(normalized.to_owned(), normalized.to_owned());
    NamespacePattern::covering(ecosystem(format), &name)
}
fn strings(patterns: &[NamespacePattern]) -> Vec<&str> {
    patterns.iter().map(NamespacePattern::as_str).collect()
}

#[test]
fn parse_normalizes_valid_patterns_per_format() {
    // Arrange
    let cases = [
        (Format::Cargo, "Acme-*"),
        (Format::Cargo, "Acme_Pay"),
        (Format::PyPi, "Acme.Payments_*"),
        (Format::NuGet, "Acme.*"),
        (Format::Maven, "com.acme.*"),
        (Format::Maven, "com.acme:core"),
        (Format::Npm, "@acme/*"),
        (Format::Npm, "@acme/ui"),
    ];

    // Act
    let actual = cases.map(|(format, raw)| {
        NamespacePattern::parse(ecosystem(format), raw)
            .map(|p| (p.as_str().to_owned(), p.is_prefix()))
    });

    // Assert
    let expected = [
        ("acme_*", true),
        ("acme_pay", false),
        ("acme-payments-*", true),
        ("acme.*", true),
        ("com.acme.*", true),
        ("com.acme:core", false),
        ("@acme/*", true),
        ("@acme/ui", false),
    ]
    .map(|(text, prefix)| Ok((text.to_owned(), prefix)));
    assert_eq!(actual, expected);
}

#[test]
fn parse_rejects_malformed_patterns() {
    // Arrange
    let cases = [
        (Format::Cargo, ""),
        (Format::Cargo, "*"),
        (Format::Cargo, "a*b"),
        (Format::Cargo, "a**"),
        (Format::Cargo, "acme*"),
        (Format::Cargo, "-*"),
        (Format::Cargo, "acme--*"),
        (Format::Cargo, "1acme-*"),
        (Format::Cargo, "ac me"),
        (Format::Npm, "acme-*"),
        (Format::Npm, "@acme/x*"),
        (Format::Npm, "@/*"),
        (Format::Npm, "@a/b/*"),
        (Format::Maven, "com.acme:*"),
        (Format::Maven, "com.acme*"),
        (Format::Maven, ".*"),
        (Format::NuGet, "acme-*"),
    ];

    // Act
    let actual =
        cases.map(|(format, raw)| NamespacePattern::parse(ecosystem(format), raw).unwrap_err());

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
            PatternError::Scope,
            PatternError::Scope,
            PatternError::EmptyBase,
            PatternError::Scope,
            PatternError::GroupId,
            PatternError::Boundary,
            PatternError::EmptyBase,
            PatternError::Boundary,
        ]
    );
}

#[test]
fn covering_lists_patterns_from_most_specific_to_most_general() {
    // Arrange
    let cases = [
        (Format::Cargo, "acme_payments_core"),
        (Format::PyPi, "acme-payments-core"),
        (Format::NuGet, "acme.payments.core"),
        (Format::Maven, "com.acme.b:c"),
        (Format::Npm, "@acme/ui"),
    ];

    // Act
    let actual = cases.map(|(format, name)| covering(format, name));

    // Assert
    let expected: [&[&str]; 5] = [
        &["acme_payments_core", "acme_payments_*", "acme_*"],
        &["acme-payments-core", "acme-payments-*", "acme-*"],
        &["acme.payments.core", "acme.payments.*", "acme.*"],
        &["com.acme.b:c", "com.acme.b.*", "com.acme.*", "com.*"],
        &["@acme/ui", "@acme/*"],
    ];
    assert_eq!(actual.each_ref().map(|p| strings(p)), expected);
}

#[test]
fn covering_does_not_cross_segment_boundaries() {
    // Arrange
    let cases = [
        (Format::Cargo, "acmecorp"),
        (Format::NuGet, "acmecorp.x"),
        (Format::PyPi, "acmecorp-x"),
        (Format::Maven, "com.acmecorp:x"),
        (Format::Maven, "com.acme:core"),
        (Format::Maven, "plain"),
        (Format::Npm, "left-pad"),
    ];

    // Act
    let actual = cases.map(|(format, name)| covering(format, name));

    // Assert
    let expected: [&[&str]; 7] = [
        &["acmecorp"],
        &["acmecorp.x", "acmecorp.*"],
        &["acmecorp-x", "acmecorp-*"],
        &["com.acmecorp:x", "com.acmecorp.*", "com.*"],
        &["com.acme:core", "com.acme.*", "com.*"],
        &["plain"],
        &["left-pad"],
    ];
    assert_eq!(actual.each_ref().map(|p| strings(p)), expected);
}

#[test]
fn cargo_name_is_covered_by_its_own_prefix_pattern() {
    // Arrange
    let name = ecosystem(Format::Cargo)
        .parse_name("Acme-Payments")
        .unwrap();
    let namespace = NamespacePattern::parse(ecosystem(Format::Cargo), "acme-*").unwrap();

    // Act
    let covering = NamespacePattern::covering(ecosystem(Format::Cargo), &name);

    // Assert
    assert_eq!(covering.last(), Some(&namespace));
}

#[test]
fn only_npm_requires_scope_for_hosted_names() {
    // Arrange
    let cases = [
        (Format::Npm, "left-pad"),
        (Format::Npm, "@acme/ui"),
        (Format::Cargo, "serde"),
        (Format::PyPi, "requests"),
        (Format::NuGet, "newtonsoft.json"),
        (Format::Maven, "com.acme:core"),
    ];

    // Act
    let actual = cases.map(|(format, name)| {
        let name = PackageName::from_parts(name.to_owned(), name.to_owned());
        hosted_name_allowed(ecosystem(format), &name)
    });

    // Assert
    assert_eq!(actual, [false, true, true, true, true, true]);
}

#[test]
fn every_format_resolves_to_its_own_ecosystem() {
    // Arrange
    let formats = [
        Format::Cargo,
        Format::Npm,
        Format::PyPi,
        Format::Maven,
        Format::NuGet,
    ];

    // Act
    let actual = formats.map(|format| ecosystem(format).format());

    // Assert
    assert_eq!(actual, formats);
}

#[test]
fn exact_npm_pattern_must_be_a_valid_package_name() {
    // Arrange
    let raws = ["@acme/ui", "JSONStream", "@/x", "a b"];

    // Act
    let actual = raws.map(|raw| NamespacePattern::parse(ecosystem(Format::Npm), raw).is_ok());

    // Assert
    assert_eq!(actual, [true, true, false, false]);
}
