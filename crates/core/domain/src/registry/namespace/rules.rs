//! Правила имён по форматам: разделители, нормализация, допустимость без scope.

use crate::registry::Format;

/// Разделители, которыми клиент может записать границу перед `*`.
pub(super) const fn accepted_separators(format: Format) -> &'static [char] {
    match format {
        Format::Cargo => &['-', '_'],
        Format::PyPi => &['-', '_', '.'],
        Format::NuGet | Format::Maven => &['.'],
        Format::Npm => &['/'],
    }
}

/// Разделитель в нормализованной форме: по нему префикс обрывается на границе сегмента.
pub(super) const fn separator(format: Format) -> char {
    match format {
        Format::Cargo => '_',
        Format::PyPi => '-',
        Format::NuGet | Format::Maven => '.',
        Format::Npm => '/',
    }
}

/// Нормализация имени или базы префикса. Для cargo повторяет `PackageName`, для остальных
/// форматов задана здесь, потому что их разбор имён ещё не поддержан.
pub(super) fn normalize(format: Format, raw: &str) -> String {
    match format {
        Format::Cargo => raw.to_ascii_lowercase().replace('-', "_"),
        // PEP 503: любая серия `-`, `_`, `.` - один дефис.
        Format::PyPi => collapse_runs(&raw.to_ascii_lowercase(), &['-', '_', '.'], '-'),
        Format::NuGet => raw.to_ascii_lowercase(),
        // Регистр значим: Maven и npm различают `Foo` и `foo`.
        Format::Maven | Format::Npm => raw.to_owned(),
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

/// npm маршрутизирует реестры только по scope, поэтому безscope-имя в hosted недопустимо.
pub(super) fn unscoped_allowed(format: Format, normalized: &str) -> bool {
    format != Format::Npm || normalized.starts_with('@')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_follows_format_rules() {
        // Arrange
        let cases = [
            (Format::Cargo, "Acme-Pay_Core"),
            (Format::PyPi, "Acme.Pay__-Core"),
            (Format::NuGet, "Acme.Pay-Core"),
            (Format::Maven, "Com.Acme:Core"),
            (Format::Npm, "@Acme/Ui"),
        ];

        // Act
        let actual = cases.map(|(format, raw)| normalize(format, raw));

        // Assert
        assert_eq!(
            actual,
            [
                "acme_pay_core",
                "acme-pay-core",
                "acme.pay-core",
                "Com.Acme:Core",
                "@Acme/Ui",
            ]
        );
    }

    #[test]
    fn only_npm_requires_scope_for_hosted_names() {
        // Arrange
        let cases = [
            (Format::Npm, "left-pad"),
            (Format::Npm, "@acme/ui"),
            (Format::Cargo, "serde"),
            (Format::PyPi, "requests"),
            (Format::NuGet, "newtonsoft.json"),
            (Format::Maven, "com.acme:core"),
        ];

        // Act
        let actual = cases.map(|(format, name)| unscoped_allowed(format, name));

        // Assert
        assert_eq!(actual, [false, true, true, true, true, true]);
    }
}
