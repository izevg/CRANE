//! Протокол NuGet-фида.

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Purl, Version, VersionError,
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
        Ok(PackageName::from_parts(normalize(raw), raw.to_owned()))
    }

    // FIXME(v0.3.0): грамматика версий NuGet — поток NuGet.
    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::NuGet))
    }

    fn prefix_base(&self, raw: &str) -> Result<String, NameError> {
        Ok(normalize(raw))
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

    // NuGet: namespace запрещён; имя печатается в написанном регистре (PURL-TYPES, nuget).
    fn purl(&self, name: &PackageName, version: Option<&Version>) -> Purl {
        Purl::new(
            "nuget",
            None,
            name.as_written().to_owned(),
            version.map(|version| version.as_written().to_owned()),
        )
    }
}

fn normalize(raw: &str) -> String {
    raw.to_ascii_lowercase()
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
    fn prefix_base_is_lowercased() {
        // Arrange
        let raw = "Acme.Pay-Core";

        // Act
        let actual = NUGET.prefix_base(raw);

        // Assert
        assert_eq!(actual, Ok("acme.pay-core".to_owned()));
    }
}
