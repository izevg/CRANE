//! Протокол Maven-репозитория.

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Version, VersionError,
};

/// Правила Maven; реализация [`Ecosystem`] без состояния.
#[derive(Debug, Clone, Copy)]
pub struct Maven;

/// Единственный экземпляр для сквозных потребителей трейта.
pub static MAVEN: Maven = Maven;

impl Ecosystem for Maven {
    fn format(&self) -> Format {
        Format::Maven
    }

    // FIXME(v0.3.0): грамматика Maven — поток Maven; пока имя только нормализуется.
    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        Ok(PackageName::from_parts(normalize(raw), raw.to_owned()))
    }

    // FIXME(v0.3.0): грамматика версий Maven — поток Maven.
    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::Maven))
    }

    // База префикса — groupId; `groupId:artifactId` в префиксе недопустим.
    fn prefix_base(&self, raw: &str) -> Result<String, NameError> {
        if raw.contains(':') {
            return Err(NameError::InvalidChar(':'));
        }
        Ok(normalize(raw))
    }

    fn separator(&self) -> char {
        '.'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['.']
    }

    fn namespace_shape(&self) -> NamespaceShape {
        NamespaceShape::Grouped
    }
}

// Регистр значим: Maven различает `Foo` и `foo`.
fn normalize(raw: &str) -> String {
    raw.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_grammar_is_reported_as_unsupported() {
        // Arrange
        let raw = "1.0.0";

        // Act
        let result = MAVEN.parse_version(raw);

        // Assert
        assert_eq!(
            result.unwrap_err(),
            VersionError::Unsupported(Format::Maven)
        );
    }

    #[test]
    fn prefix_base_is_a_group_id_without_artifact() {
        // Arrange
        let raws = ["Com.Acme", "com.acme:core"];

        // Act
        let actual = raws.map(|raw| MAVEN.prefix_base(raw));

        // Assert
        assert_eq!(
            actual,
            [Ok("Com.Acme".to_owned()), Err(NameError::InvalidChar(':'))]
        );
    }
}
