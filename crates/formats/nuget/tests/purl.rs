use crane_domain::registry::{Ecosystem, OrderKeyBuilder, Version};
use crane_format_nuget::NUGET;

fn version(raw: &str) -> Version {
    Version::from_parts(
        raw.to_owned(),
        raw.to_owned(),
        OrderKeyBuilder::new().finish(),
        false,
    )
}

#[test]
fn purl_keeps_written_name_and_has_no_namespace() {
    // Arrange
    // Первый кейс — tests/types/nuget-test.json репозитория package-url/purl-spec: точки
    // в имени остаются в имени, namespace у NuGet запрещён.
    let cases = [
        ("EnterpriseLibrary.Common", Some("6.0.1304")),
        ("Newtonsoft.Json", None),
    ];

    // Act
    let actual = cases.map(|(name, raw)| {
        let name = NUGET.parse_name(name).expect("имя разбирается");
        NUGET.purl(&name, raw.map(version).as_ref()).to_string()
    });

    // Assert
    assert_eq!(
        actual,
        [
            "pkg:nuget/EnterpriseLibrary.Common@6.0.1304",
            "pkg:nuget/Newtonsoft.Json",
        ]
    );
}
