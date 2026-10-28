use crane_domain::registry::{Ecosystem, OrderKeyBuilder, Version};
use crane_format_pypi::PYPI;

fn version(raw: &str) -> Version {
    Version::from_parts(
        raw.to_owned(),
        raw.to_owned(),
        OrderKeyBuilder::new().finish(),
        false,
    )
}

#[test]
fn purl_lowercases_name_and_replaces_underscore_with_dash_but_keeps_dots() {
    // Arrange
    // Первые три кейса — tests/types/pypi-test.json репозитория package-url/purl-spec.
    // Точка остаётся: спецификация заменяет только `_`, а PEP 503 свернул бы и точки.
    let cases = [
        ("Django_package", Some("1.11.1.dev1")),
        ("django", Some("1.11.1")),
        ("django-allauth", Some("12.23")),
        ("Zope.Interface", None),
    ];

    // Act
    let actual = cases.map(|(name, raw)| {
        let name = PYPI.parse_name(name).expect("имя разбирается");
        PYPI.purl(&name, raw.map(version).as_ref()).to_string()
    });

    // Assert
    assert_eq!(
        actual,
        [
            "pkg:pypi/django-package@1.11.1.dev1",
            "pkg:pypi/django@1.11.1",
            "pkg:pypi/django-allauth@12.23",
            "pkg:pypi/zope.interface",
        ]
    );
}
