#![allow(
    clippy::unwrap_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::registry::Ecosystem;

use crate::ecosystem::CARGO;

#[test]
fn purl_keeps_written_name_has_no_namespace_and_encodes_build_metadata() {
    // Arrange
    // Первые три кейса — tests/types/cargo-test.json репозитория package-url/purl-spec.
    let cases = [
        ("rand", Some("0.7.2")),
        ("clap", Some("2.33.0")),
        ("structopt", Some("0.3.11")),
        ("Foo_Bar", Some("1.2.3-rc.1+build5")),
        ("serde", None),
    ];

    // Act
    let actual = cases.map(|(name, version)| {
        let name = CARGO.parse_name(name).unwrap();
        let version = version.map(|raw| CARGO.parse_version(raw).unwrap());
        CARGO.purl(&name, version.as_ref()).to_string()
    });

    // Assert
    assert_eq!(
        actual,
        [
            "pkg:cargo/rand@0.7.2",
            "pkg:cargo/clap@2.33.0",
            "pkg:cargo/structopt@0.3.11",
            "pkg:cargo/Foo_Bar@1.2.3-rc.1%2Bbuild5",
            "pkg:cargo/serde",
        ]
    );
}
