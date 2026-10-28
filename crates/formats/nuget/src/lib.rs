//! Протокол NuGet-фида.

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Version, VersionError,
};

/// Правила `NuGet`; реализация [`Ecosystem`] без состояния.
#[derive(Debug, Clone, Copy)]
pub struct NuGet;

/// Единственный экземпляр для сквозных потребителей трейта.
pub static NUGET: NuGet = NuGet;

impl Ecosystem for NuGet {
    fn format(&self) -> Format {
        Format::NuGet
    }

    // FIXME(v0.3.0): грамматика NuGet — поток NuGet; пока имя только нормализуется.
    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        Ok(PackageName::from_parts(self.normalize(raw), raw.to_owned()))
    }

    // FIXME(v0.3.0): грамматика версий NuGet — поток NuGet.
    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::NuGet))
    }

    fn normalize(&self, raw: &str) -> String {
        raw.to_ascii_lowercase()
    }

    fn separator(&self) -> char {
        '.'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['.']
    }

    fn namespace_shape(&self) -> NamespaceShape {
        NamespaceShape::Delimited
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
        let result = NUGET.parse_version(raw);

        // Assert
        assert_eq!(
            result.unwrap_err(),
            VersionError::Unsupported(Format::NuGet)
        );
    }

    #[test]
    fn normalize_follows_format_rules() {
        // Arrange
        let cases = [("Acme.Pay-Core", "acme.pay-core")];

        // Act
        let actual = cases.map(|(raw, _)| NUGET.normalize(raw));

        // Assert
        assert_eq!(actual, cases.map(|(_, expected)| expected));
    }
}
