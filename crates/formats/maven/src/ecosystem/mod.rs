//! Экосистема Maven: `groupId:artifactId`, версии по `GenericVersionScheme` и timestamp-сборки SNAPSHOT.

mod generic;
mod name;
mod version;

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Purl, Version, VersionError,
};

pub use version::Build;

/// Правила Maven; реализация [`Ecosystem`] без состояния.
#[derive(Debug, Clone, Copy)]
pub struct Maven;

/// Единственный экземпляр для сквозных потребителей трейта.
pub static MAVEN: Maven = Maven;

impl Ecosystem for Maven {
    fn format(&self) -> Format {
        Format::Maven
    }

    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        name::parse_name(raw)
    }

    fn parse_version(&self, raw: &str) -> Result<Version, VersionError> {
        version::parse_version(raw)
    }

    // База префикса — groupId; `groupId:artifactId` в префиксе недопустим.
    // Регистр значим: Maven различает `Foo` и `foo`.
    fn prefix_base(&self, raw: &str) -> Result<String, NameError> {
        if raw.contains(':') {
            return Err(NameError::InvalidChar(':'));
        }
        Ok(raw.to_owned())
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

    // Maven: имя пакета хранится как `groupId:artifactId`; groupId — namespace, регистр значим
    // (PURL-TYPES, maven). Нормализованная форма и каноническая версия равны написанным.
    fn purl(&self, name: &PackageName, version: Option<&Version>) -> Purl {
        let (namespace, name) = match name.normalized().split_once(':') {
            Some((group, artifact)) => (Some(group.to_owned()), artifact),
            None => (None, name.normalized()),
        };
        Purl::new(
            "maven",
            namespace,
            name.to_owned(),
            version.map(|version| version.canonical().to_owned()),
        )
    }
}

#[cfg(test)]
mod tests;
