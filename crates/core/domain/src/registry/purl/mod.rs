//! Идентификатор пакета по спецификации package-url (ECMA-427, purl-spec): ключ findings, графа
//! зависимостей и событий SIEM. Тип и разбиение имени на namespace и name задаёт экосистема.

use std::fmt::{self, Write as _};

/// Package URL без квалификаторов и subpath. Значения хранятся сырыми, кодируются при печати:
/// так экосистема не знает про процентное кодирование, а печать одна на все типы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purl {
    ty: &'static str,
    namespace: Option<String>,
    name: String,
    version: Option<String>,
}

impl Purl {
    /// `ty` — тип purl в нижнем регистре (`cargo`, `npm`); namespace из нескольких сегментов
    /// записывается через `/`.
    #[must_use]
    pub fn new(
        ty: &'static str,
        namespace: Option<String>,
        name: String,
        version: Option<String>,
    ) -> Self {
        Self {
            ty,
            namespace,
            name,
            version,
        }
    }
}

/// Печатает компонент по разделу о кодировании символов: кодируется всё, кроме букв, цифр,
/// `.-_~` и двоеточия (оно в purl не кодируется никогда). `@`, `/`, `+`, `?`, `#` и пробел
/// внутри компонента — данные, а не разделители, поэтому кодируются.
fn write_encoded(f: &mut fmt::Formatter<'_>, component: &str) -> fmt::Result {
    for byte in component.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'~' | b':') {
            f.write_char(char::from(byte))?;
        } else {
            // Шестнадцатеричные цифры в верхнем регистре: каноническая форма purl-spec.
            write!(f, "%{byte:02X}")?;
        }
    }
    Ok(())
}

impl fmt::Display for Purl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pkg:{}/", self.ty)?;
        // Ведущие, замыкающие и повторные `/` в namespace не значимы (пустых сегментов нет).
        let segments = self
            .namespace
            .iter()
            .flat_map(|namespace| namespace.split('/'))
            .filter(|segment| !segment.is_empty());
        for segment in segments {
            write_encoded(f, segment)?;
            f.write_char('/')?;
        }
        write_encoded(f, &self.name)?;
        if let Some(version) = &self.version {
            f.write_char('@')?;
            write_encoded(f, version)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn purl(
        ty: &'static str,
        namespace: Option<&str>,
        name: &str,
        version: Option<&str>,
    ) -> String {
        Purl::new(
            ty,
            namespace.map(str::to_owned),
            name.to_owned(),
            version.map(str::to_owned),
        )
        .to_string()
    }

    #[test]
    fn display_prints_canonical_form_with_and_without_namespace_and_version() {
        // Arrange
        // Источник: tests/types/{cargo,npm,maven}-test.json репозитория package-url/purl-spec.
        let cases = [
            ("cargo", None, "rand", Some("0.7.2")),
            ("maven", Some("org.apache.commons"), "io", None),
            (
                "maven",
                Some("org.apache.xmlgraphics"),
                "batik-anim",
                Some("1.9.1"),
            ),
        ];

        // Act
        let actual = cases.map(|(ty, namespace, name, version)| purl(ty, namespace, name, version));

        // Assert
        assert_eq!(
            actual,
            [
                "pkg:cargo/rand@0.7.2",
                "pkg:maven/org.apache.commons/io",
                "pkg:maven/org.apache.xmlgraphics/batik-anim@1.9.1",
            ]
        );
    }

    #[test]
    fn display_encodes_every_namespace_segment_and_keeps_slash_between_them() {
        // Arrange
        let cases = [
            Some("@angular"),
            Some("org.acme/team a/ü"),
            Some("/a//b/"),
            Some(""),
        ];

        // Act
        let actual = cases.map(|namespace| purl("npm", namespace, "n", None));

        // Assert
        assert_eq!(
            actual,
            [
                "pkg:npm/%40angular/n",
                "pkg:npm/org.acme/team%20a/%C3%BC/n",
                "pkg:npm/a/b/n",
                "pkg:npm/n",
            ]
        );
    }

    #[test]
    fn display_encodes_data_characters_in_name_and_version() {
        // Arrange
        // `@` и `/` внутри имени — данные, а не разделители; `+` — метаданные сборки semver;
        // пробел — `%20`; не-ASCII — по байтам UTF-8; `.-_~` и `:` остаются как есть.
        let cases = [
            ("a@b/c", Some("1.0.0")),
            ("pkg", Some("1.2.3-rc.1+build5")),
            ("pkg", Some("1.0.0 Final")),
            ("пакет", Some("1~2_3:4")),
            ("a?b#c=d&e", None),
        ];

        // Act
        let actual = cases.map(|(name, version)| purl("x", None, name, version));

        // Assert
        assert_eq!(
            actual,
            [
                "pkg:x/a%40b%2Fc@1.0.0",
                "pkg:x/pkg@1.2.3-rc.1%2Bbuild5",
                "pkg:x/pkg@1.0.0%20Final",
                "pkg:x/%D0%BF%D0%B0%D0%BA%D0%B5%D1%82@1~2_3:4",
                "pkg:x/a%3Fb%23c%3Dd%26e",
            ]
        );
    }
}
