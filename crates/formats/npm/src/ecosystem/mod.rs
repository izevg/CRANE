//! Экосистема npm: грамматика имён (в том числе старые имена с заглавными буквами) и `SemVer` 2.0.

mod name;
mod version;

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Version, VersionError,
};

pub(crate) use name::parse_name;
pub(crate) use version::parse_version;

/// Правила npm; реализация [`Ecosystem`] без состояния.
#[derive(Debug, Clone, Copy)]
pub struct Npm;

/// Единственный экземпляр для сквозных потребителей трейта.
pub static NPM: Npm = Npm;

impl Ecosystem for Npm {
    fn format(&self) -> Format {
        Format::Npm
    }

    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        parse_name(raw)
    }

    fn parse_version(&self, raw: &str) -> Result<Version, VersionError> {
        parse_version(raw)
    }

    // Префикс npm задаётся только scope (`NamespaceShape::Scoped`), базу `NamespacePattern`
    // не запрашивает; регистр значим: npm различает `Foo` и `foo`.
    fn prefix_base(&self, raw: &str) -> Result<String, NameError> {
        Ok(raw.to_owned())
    }

    fn separator(&self) -> char {
        '/'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['/']
    }

    // Клиент маршрутизирует реестры только по scope, безscope-имя нечем защитить от подмены.
    fn namespace_shape(&self) -> NamespaceShape {
        NamespaceShape::Scoped
    }
}

#[cfg(test)]
mod tests;
