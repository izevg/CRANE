//! Протокол PyPI-индекса.

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespaceShape, PackageName, Purl, Version, VersionError,
};

/// Правила `PyPI`; реализация [`Ecosystem`] без состояния.
#[derive(Debug, Clone, Copy)]
pub struct PyPi;

/// Единственный экземпляр для сквозных потребителей трейта.
pub static PYPI: PyPi = PyPi;

impl Ecosystem for PyPi {
    fn format(&self) -> Format {
        Format::PyPi
    }

    // FIXME(v0.3.0): грамматика PyPI — поток PyPI; пока имя только нормализуется.
    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        Ok(PackageName::from_parts(normalize(raw), raw.to_owned()))
    }

    // FIXME(v0.3.0): грамматика версий PyPI — поток PyPI.
    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::PyPi))
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

fn collapse_runs(text: &str, run: &[char], replacement: char) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_run = false;
    for c in text.chars() {
        if run.contains(&c) {
            if !in_run {
                out.push(replacement);
            }
            in_run = true;
        } else {
            out.push(c);
            in_run = false;
        }
    }
    out
}

// PEP 503: любая серия `-`, `_`, `.` — один дефис.
fn normalize(raw: &str) -> String {
    collapse_runs(&raw.to_ascii_lowercase(), &['-', '_', '.'], '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_grammar_is_reported_as_unsupported() {
        // Arrange
        let raw = "1.0.0";

        // Act
        let result = PYPI.parse_version(raw);

        // Assert
        assert_eq!(result.unwrap_err(), VersionError::Unsupported(Format::PyPi));
    }

    #[test]
    fn prefix_base_folds_runs_of_separators_and_drops_the_boundary_one() {
        // Arrange
        let raws = ["Acme.Pay__-Core", "acme-_", "-"];

        // Act
        let actual = raws.map(|raw| PYPI.prefix_base(raw));

        // Assert
        assert_eq!(
            actual,
            [
                Ok("acme-pay-core".to_owned()),
                Ok("acme".to_owned()),
                Err(NameError::Empty)
            ]
        );
    }
}
