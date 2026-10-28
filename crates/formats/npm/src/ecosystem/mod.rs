//! Экосистема npm: грамматика имён (в том числе старые имена с заглавными буквами) и `SemVer` 2.0.

mod name;
mod version;

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Purl, Version, VersionError,
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

    // npm: namespace — scope вместе с `@` (при печати он станет `%40`); регистр значим,
    // старые имена с заглавными буквами не приводятся к нижнему (PURL-TYPES, npm).
    fn purl(&self, name: &PackageName, version: Option<&Version>) -> Purl {
        // В валидном имени `/` бывает только после scope.
        let written = name.as_written();
        let (namespace, name) = written
            .split_once('/')
            .map_or((None, written), |(scope, rest)| {
                (Some(scope.to_owned()), rest)
            });
        Purl::new(
            "npm",
            namespace,
            name.to_owned(),
            version.map(|version| version.as_written().to_owned()),
        )
    }
}

#[cfg(test)]
mod tests;
