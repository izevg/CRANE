use crane_domain::registry::{Ecosystem, OrderKeyBuilder, Version};
use crane_format_maven::MAVEN;

fn version(raw: &str) -> Version {
    Version::from_parts(
        raw.to_owned(),
        raw.to_owned(),
        OrderKeyBuilder::new().finish(),
        false,
    )
}

#[test]
fn purl_takes_group_id_as_namespace_and_artifact_id_as_name() {
    // Arrange
    // Кейсы с версией и без неё, регистр и пробел в версии — tests/types/maven-test.json
    // репозитория package-url/purl-spec. Имя без `:` грамматика отвергает, поэтому кейса «solo» нет.
    let cases = [
        ("org.apache.commons:io", Some("1.3.4")),
        ("org.apache.commons:io", None),
        ("HTTPClient:HTTPClient", Some("0.3-3")),
        ("mygroup:myartifact", Some("1.0.0 Final")),
    ];

    // Act
    let actual = cases.map(|(name, raw)| {
        let name = MAVEN.parse_name(name).expect("имя разбирается");
        MAVEN.purl(&name, raw.map(version).as_ref()).to_string()
    });

    // Assert
    assert_eq!(
        actual,
        [
            "pkg:maven/org.apache.commons/io@1.3.4",
            "pkg:maven/org.apache.commons/io",
            "pkg:maven/HTTPClient/HTTPClient@0.3-3",
            "pkg:maven/mygroup/myartifact@1.0.0%20Final",
        ]
    );
}
