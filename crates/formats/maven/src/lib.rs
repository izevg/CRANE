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
        Ok(PackageName::from_parts(self.normalize(raw), raw.to_owned()))
    }

    // FIXME(v0.3.0): грамматика версий Maven — поток Maven.
    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::Maven))
    }

    // Регистр значим: Maven различает `Foo` и `foo`.
    fn normalize(&self, raw: &str) -> String {
        raw.to_owned()
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
    fn normalize_follows_format_rules() {
        // Arrange
        let cases = [("Com.Acme:Core", "Com.Acme:Core")];

        // Act
        let actual = cases.map(|(raw, _)| MAVEN.normalize(raw));

        // Assert
        assert_eq!(actual, cases.map(|(_, expected)| expected));
    }
}
