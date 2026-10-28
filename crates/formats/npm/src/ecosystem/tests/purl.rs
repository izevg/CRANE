#![allow(
    clippy::unwrap_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::registry::Ecosystem;

use crate::ecosystem::NPM;

#[test]
fn purl_splits_scope_into_encoded_namespace_and_keeps_name_case() {
    // Arrange
    // Кейсы foobar и @angular/animation — tests/types/npm-test.json репозитория
    // package-url/purl-spec; регистр значим: старые имена с заглавными буквами не понижаются.
    let cases = [
        ("foobar", Some("12.3.1")),
        ("@angular/animation", Some("12.3.1")),
        ("@babel/core", None),
        ("JSONStream", Some("1.3.5")),
    ];

    // Act
    let actual = cases.map(|(name, version)| {
        let name = NPM.parse_name(name).unwrap();
        let version = version.map(|raw| NPM.parse_version(raw).unwrap());
        NPM.purl(&name, version.as_ref()).to_string()
    });

    // Assert
    assert_eq!(
        actual,
        [
            "pkg:npm/foobar@12.3.1",
            "pkg:npm/%40angular/animation@12.3.1",
            "pkg:npm/%40babel/core",
            "pkg:npm/JSONStream@1.3.5",
        ]
    );
}
