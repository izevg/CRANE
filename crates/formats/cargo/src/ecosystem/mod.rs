//! Экосистема Cargo: грамматика имён crates.io и версии `SemVer` 2.0.

mod name;
mod version;

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Version, VersionError,
};

pub(crate) use name::parse_name;
pub(crate) use version::parse_version;

/// Правила Cargo; реализация [`Ecosystem`] без состояния.
#[derive(Debug, Clone, Copy)]
pub struct Cargo;

/// Единственный экземпляр для сквозных потребителей трейта.
pub static CARGO: Cargo = Cargo;

impl Ecosystem for Cargo {
    fn format(&self) -> Format {
        Format::Cargo
    }

    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        parse_name(raw)
    }

    fn parse_version(&self, raw: &str) -> Result<Version, VersionError> {
        parse_version(raw)
    }

    fn prefix_base(&self, raw: &str) -> Result<String, NameError> {
        parse_name(raw).map(|name| name.normalized().to_owned())
    }

    fn separator(&self) -> char {
        '_'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['-', '_']
    }

    fn namespace_shape(&self) -> NamespaceShape {
        NamespaceShape::Delimited
    }
}

#[cfg(test)]
mod tests;
