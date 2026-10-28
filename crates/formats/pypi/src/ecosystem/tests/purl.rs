#![allow(
    clippy::unwrap_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::registry::Ecosystem;

use crate::ecosystem::PYPI;

#[test]
fn purl_lowercases_the_name_turns_underscores_into_dashes_and_keeps_dots_and_written_version() {
    // Arrange
    // Первые два кейса — tests/types/pypi-test.json репозитория package-url/purl-spec.
    let cases = [
        ("django", Some("1.11.1")),
        ("Django_package", Some("1.11.1.post1")),
        ("Foo.Bar", Some("1.0.0")),
        ("requests", None),
    ];

    // Act
    let actual = cases.map(|(name, version)| {
        let name = PYPI.parse_name(name).unwrap();
        let version = version.map(|raw| PYPI.parse_version(raw).unwrap());
        PYPI.purl(&name, version.as_ref()).to_string()
    });

    // Assert
    assert_eq!(
        actual,
        [
            "pkg:pypi/django@1.11.1",
            "pkg:pypi/django-package@1.11.1.post1",
            "pkg:pypi/foo.bar@1.0.0",
            "pkg:pypi/requests",
        ]
    );
}
