//! Протокол PyPI-индекса.

use crane_domain::registry::{Ecosystem, Format, NameError, PackageName, Version, VersionError};

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
        Ok(PackageName::from_parts(self.normalize(raw), raw.to_owned()))
    }

    // FIXME(v0.3.0): грамматика версий PyPI — поток PyPI.
    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::PyPi))
    }

    // PEP 503: любая серия `-`, `_`, `.` — один дефис.
    fn normalize(&self, raw: &str) -> String {
        collapse_runs(&raw.to_ascii_lowercase(), &['-', '_', '.'], '-')
    }

    fn separator(&self) -> char {
        '-'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['-', '_', '.']
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
    fn normalize_follows_format_rules() {
        // Arrange
        let cases = [
            ("Acme.Pay__-Core", "acme-pay-core"),
            ("Requests", "requests"),
        ];

        // Act
        let actual = cases.map(|(raw, _)| PYPI.normalize(raw));

        // Assert
        assert_eq!(actual, cases.map(|(_, expected)| expected));
    }
}
