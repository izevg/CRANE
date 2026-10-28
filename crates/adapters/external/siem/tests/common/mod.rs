//! Тестовая экосистема: `siem` не зависит от `formats`, а проверять нужно только то, что
//! приёмник берёт purl у экосистемы, а не собирает его сам по тегу формата.

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Purl, Version, VersionError,
};

struct Probe;

impl Ecosystem for Probe {
    fn format(&self) -> Format {
        Format::Cargo
    }

    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        Ok(PackageName::from_parts(raw.to_owned(), raw.to_owned()))
    }

    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::Cargo))
    }

    fn prefix_base(&self, raw: &str) -> Result<String, NameError> {
        Ok(raw.to_owned())
    }

    fn separator(&self) -> char {
        '-'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['-']
    }

    fn namespace_shape(&self) -> NamespaceShape {
        NamespaceShape::Delimited
    }

    // Тип `probe` и namespace `ns` не совпадают ни с одним форматом: так видно, что строку
    // собрала экосистема, а не приёмник по `Format`.
    fn purl(&self, name: &PackageName, version: Option<&Version>) -> Purl {
        Purl::new(
            "probe",
            Some("ns".to_owned()),
            name.as_written().to_owned(),
            version.map(|version| version.as_written().to_owned()),
        )
    }
}

pub(crate) fn ecosystems(_format: Format) -> &'static dyn Ecosystem {
    &Probe
}
