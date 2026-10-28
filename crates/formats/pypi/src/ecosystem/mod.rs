//! Экосистема `PyPI`: имена PEP 508 с нормализацией PEP 503 и версии PEP 440.

mod name;
mod version;

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Purl, Version, VersionError,
};

pub(crate) use name::{normalize, parse_name};
pub(crate) use version::parse_version;

/// Правила `PyPI`; реализация [`Ecosystem`] без состояния.
#[derive(Debug, Clone, Copy)]
pub struct PyPi;

/// Единственный экземпляр для сквозных потребителей трейта.
pub static PYPI: PyPi = PyPi;

impl Ecosystem for PyPi {
    fn format(&self) -> Format {
        Format::PyPi
    }

    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        parse_name(raw)
    }

    fn parse_version(&self, raw: &str) -> Result<Version, VersionError> {
        parse_version(raw)
    }

    // PEP 503: серия `-`, `_`, `.` — один дефис, в том числе на границе (`acme-_*`): дефис
    // между базой и `*` добавляет сам паттерн.
    fn prefix_base(&self, raw: &str) -> Result<String, NameError> {
        let folded = normalize(raw);
        let base = folded.trim_end_matches('-');
        if base.is_empty() {
            return Err(NameError::Empty);
        }
        Ok(base.to_owned())
    }

    fn separator(&self) -> char {
        '-'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['-', '_', '.']
    }

    fn namespace_shape(&self) -> NamespaceShape {
        NamespaceShape::Delimited
    }

    // PyPI: purl-spec требует нижний регистр и `_` → `-`; точка остаётся как есть, поэтому
    // нормализованная форма PEP 503 (она сворачивает и точки) здесь не годится.
    fn purl(&self, name: &PackageName, version: Option<&Version>) -> Purl {
        Purl::new(
            "pypi",
            None,
            name.as_written().to_ascii_lowercase().replace('_', "-"),
            version.map(|version| version.as_written().to_owned()),
        )
    }
}

#[cfg(test)]
mod tests;
