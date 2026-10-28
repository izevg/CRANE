//! Паттерн namespace: разбор, нормализация и список паттернов, покрывающих имя.

use super::super::{Format, NameError, PackageName};
use super::rules::{accepted_separators, normalize, separator};

const WILDCARD: char = '*';

/// Почему строка не годится в паттерн namespace.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
    #[error("паттерн пустой")]
    Empty,
    #[error("недопустимый символ {0:?}: разрешены только видимые ASCII без пробелов")]
    Charset(char),
    #[error("`*` допустим один раз и только последним символом")]
    Wildcard,
    #[error("перед `*` нет базы паттерна")]
    EmptyBase,
    #[error("`*` должен стоять сразу после разделителя формата и ровно одного")]
    Boundary,
    #[error("npm: префикс допустим только в виде `@scope/*`")]
    Scope,
    #[error("maven: префикс задаётся только по groupId, без `:`")]
    GroupId,
    #[error("имя в паттерне недопустимо: {0}")]
    Name(#[from] NameError),
}

/// Нормализованный паттерн: точное имя или префикс, оканчивающийся разделителем формата и `*`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamespacePattern(String);

impl NamespacePattern {
    /// Разбирает паттерн из конфигурации по правилам формата.
    ///
    /// Префикс обязан оканчиваться разделителем: `acme-*` покрывает `acme-core`, но не
    /// `acmecorp`, иначе чужой пакет с общим началом имени оказался бы «внутренним».
    pub fn parse(format: Format, raw: &str) -> Result<Self, PatternError> {
        if raw.is_empty() {
            return Err(PatternError::Empty);
        }
        if let Some(bad) = raw.chars().find(|c| !c.is_ascii_graphic()) {
            return Err(PatternError::Charset(bad));
        }
        match raw.matches(WILDCARD).count() {
            0 => Self::parse_exact(format, raw),
            1 if raw.ends_with(WILDCARD) => Self::parse_prefix(format, raw),
            _ => Err(PatternError::Wildcard),
        }
    }

    fn parse_exact(format: Format, raw: &str) -> Result<Self, PatternError> {
        let normalized = match format {
            Format::Cargo => PackageName::parse(format, raw)?.normalized().to_owned(),
            Format::Maven | Format::Npm | Format::PyPi | Format::NuGet => normalize(format, raw),
        };
        Ok(Self(normalized))
    }

    fn parse_prefix(format: Format, raw: &str) -> Result<Self, PatternError> {
        // Срез по ASCII-символу `*` в конце: граница символа гарантирована.
        let head = raw.trim_end_matches(WILDCARD);
        match format {
            Format::Npm => Self::parse_scope(head),
            Format::Maven | Format::Cargo | Format::PyPi | Format::NuGet => {
                Self::parse_delimited(format, head)
            }
        }
    }

    /// npm: только `@scope/`, без вложенных сегментов.
    fn parse_scope(head: &str) -> Result<Self, PatternError> {
        let Some(scope) = head
            .strip_prefix('@')
            .and_then(|rest| rest.strip_suffix('/'))
        else {
            return Err(PatternError::Scope);
        };
        if scope.is_empty() {
            return Err(PatternError::EmptyBase);
        }
        if scope.contains('/') {
            return Err(PatternError::Scope);
        }
        Ok(Self(format!("{head}{WILDCARD}")))
    }

    fn parse_delimited(format: Format, head: &str) -> Result<Self, PatternError> {
        let Some(last) = head.chars().last() else {
            return Err(PatternError::EmptyBase);
        };
        if format == Format::Maven && head.contains(':') {
            return Err(PatternError::GroupId);
        }
        if !accepted_separators(format).contains(&last) {
            return Err(PatternError::Boundary);
        }
        let normalized = normalize(format, head);
        let sep = separator(format);
        let base = normalized.strip_suffix(sep).unwrap_or(&normalized);
        if base.is_empty() {
            return Err(PatternError::EmptyBase);
        }
        // Двойной разделитель делает бессмысленную базу `acme_` и паттерн, которому
        // не соответствует ни одно имя, написанное оператором «на глаз».
        if base.ends_with(sep) {
            return Err(PatternError::Boundary);
        }
        if format == Format::Cargo {
            let raw_base = head.get(..head.len() - last.len_utf8()).unwrap_or_default();
            PackageName::parse(format, raw_base)?;
        }
        Ok(Self(format!("{normalized}{WILDCARD}")))
    }

    /// Точное имя как паттерн.
    #[must_use]
    pub fn exact(name: &PackageName) -> Self {
        Self(name.normalized().to_owned())
    }

    /// Все паттерны, которые покрыли бы имя, от самого специфичного к самому общему.
    #[must_use]
    pub fn covering(format: Format, name: &PackageName) -> Vec<Self> {
        covering_of(format, name.normalized())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn is_prefix(&self) -> bool {
        self.0.ends_with(WILDCARD)
    }
}

/// То же, что [`NamespacePattern::covering`], над нормализованной строкой: так границы
/// проверяются для всех пяти форматов, хотя разбор `PackageName` поддержан не для всех.
fn covering_of(format: Format, normalized: &str) -> Vec<NamespacePattern> {
    let mut patterns = vec![NamespacePattern(normalized.to_owned())];
    match format {
        Format::Npm => patterns.extend(scope_prefix(normalized)),
        Format::Maven => {
            // Имя без `:` не принадлежит никакому groupId: покрыть его может только точный паттерн.
            if let Some((group, _)) = normalized.split_once(':') {
                patterns.extend(group_prefixes(group));
            }
        }
        Format::Cargo | Format::PyPi | Format::NuGet => {
            patterns.extend(prefixes(normalized, separator(format)));
        }
    }
    patterns
}

fn scope_prefix(normalized: &str) -> Option<NamespacePattern> {
    if !normalized.starts_with('@') {
        return None;
    }
    let (scope, _) = normalized.split_once('/')?;
    // `@/x` без имени scope паттерна не имеет.
    (scope.len() > 1).then(|| NamespacePattern(format!("{scope}/{WILDCARD}")))
}

/// `G.*` покрывает и сам groupId `G`, и `G.x`; затем префиксы родительских групп.
fn group_prefixes(group: &str) -> Vec<NamespacePattern> {
    if group.is_empty() {
        return Vec::new();
    }
    let own = NamespacePattern(format!("{group}.{WILDCARD}"));
    std::iter::once(own).chain(prefixes(group, '.')).collect()
}

/// Префиксы `<начало><sep>*` справа налево по каждому разделителю, кроме нулевой позиции.
fn prefixes(text: &str, sep: char) -> Vec<NamespacePattern> {
    text.char_indices()
        .rev()
        .filter(|&(index, c)| c == sep && index > 0)
        .filter_map(|(index, c)| text.get(..index + c.len_utf8()))
        .map(|head| NamespacePattern(format!("{head}{WILDCARD}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(patterns: &[NamespacePattern]) -> Vec<&str> {
        patterns.iter().map(NamespacePattern::as_str).collect()
    }

    #[test]
    fn parse_normalizes_valid_patterns_per_format() {
        // Arrange
        let cases = [
            (Format::Cargo, "Acme-*"),
            (Format::Cargo, "Acme_Pay"),
            (Format::PyPi, "Acme.Payments_*"),
            (Format::NuGet, "Acme.*"),
            (Format::Maven, "com.acme.*"),
            (Format::Maven, "com.acme:core"),
            (Format::Npm, "@acme/*"),
            (Format::Npm, "@acme/ui"),
        ];

        // Act
        let actual = cases.map(|(format, raw)| {
            NamespacePattern::parse(format, raw).map(|p| (p.as_str().to_owned(), p.is_prefix()))
        });

        // Assert
        let expected = [
            ("acme_*", true),
            ("acme_pay", false),
            ("acme-payments-*", true),
            ("acme.*", true),
            ("com.acme.*", true),
            ("com.acme:core", false),
            ("@acme/*", true),
            ("@acme/ui", false),
        ]
        .map(|(text, prefix)| Ok((text.to_owned(), prefix)));
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_rejects_malformed_patterns() {
        // Arrange
        let cases = [
            (Format::Cargo, ""),
            (Format::Cargo, "*"),
            (Format::Cargo, "a*b"),
            (Format::Cargo, "a**"),
            (Format::Cargo, "acme*"),
            (Format::Cargo, "-*"),
            (Format::Cargo, "acme--*"),
            (Format::Cargo, "1acme-*"),
            (Format::Cargo, "ac me"),
            (Format::Npm, "acme-*"),
            (Format::Npm, "@acme/x*"),
            (Format::Npm, "@/*"),
            (Format::Npm, "@a/b/*"),
            (Format::Maven, "com.acme:*"),
            (Format::Maven, "com.acme*"),
            (Format::Maven, ".*"),
            (Format::NuGet, "acme-*"),
        ];

        // Act
        let actual = cases.map(|(format, raw)| NamespacePattern::parse(format, raw).unwrap_err());

        // Assert
        assert_eq!(
            actual,
            [
                PatternError::Empty,
                PatternError::EmptyBase,
                PatternError::Wildcard,
                PatternError::Wildcard,
                PatternError::Boundary,
                PatternError::EmptyBase,
                PatternError::Boundary,
                PatternError::Name(NameError::InvalidStart('1')),
                PatternError::Charset(' '),
                PatternError::Scope,
                PatternError::Scope,
                PatternError::EmptyBase,
                PatternError::Scope,
                PatternError::GroupId,
                PatternError::Boundary,
                PatternError::EmptyBase,
                PatternError::Boundary,
            ]
        );
    }

    #[test]
    fn covering_lists_patterns_from_most_specific_to_most_general() {
        // Arrange
        let cases = [
            (Format::Cargo, "acme_payments_core"),
            (Format::PyPi, "acme-payments-core"),
            (Format::NuGet, "acme.payments.core"),
            (Format::Maven, "com.acme.b:c"),
            (Format::Npm, "@acme/ui"),
        ];

        // Act
        let actual = cases.map(|(format, name)| covering_of(format, name));

        // Assert
        let expected: [&[&str]; 5] = [
            &["acme_payments_core", "acme_payments_*", "acme_*"],
            &["acme-payments-core", "acme-payments-*", "acme-*"],
            &["acme.payments.core", "acme.payments.*", "acme.*"],
            &["com.acme.b:c", "com.acme.b.*", "com.acme.*", "com.*"],
            &["@acme/ui", "@acme/*"],
        ];
        assert_eq!(actual.each_ref().map(|p| strings(p)), expected);
    }

    #[test]
    fn covering_does_not_cross_segment_boundaries() {
        // Arrange
        let cases = [
            (Format::Cargo, "acmecorp"),
            (Format::NuGet, "acmecorp.x"),
            (Format::PyPi, "acmecorp-x"),
            (Format::Maven, "com.acmecorp:x"),
            (Format::Maven, "com.acme:core"),
            (Format::Maven, "plain"),
            (Format::Npm, "left-pad"),
        ];

        // Act
        let actual = cases.map(|(format, name)| covering_of(format, name));

        // Assert
        let expected: [&[&str]; 7] = [
            &["acmecorp"],
            &["acmecorp.x", "acmecorp.*"],
            &["acmecorp-x", "acmecorp-*"],
            &["com.acmecorp:x", "com.acmecorp.*", "com.*"],
            &["com.acme:core", "com.acme.*", "com.*"],
            &["plain"],
            &["left-pad"],
        ];
        assert_eq!(actual.each_ref().map(|p| strings(p)), expected);
    }

    #[test]
    fn cargo_name_is_covered_by_its_own_prefix_pattern() {
        // Arrange
        let name = PackageName::parse(Format::Cargo, "Acme-Payments").unwrap();
        let namespace = NamespacePattern::parse(Format::Cargo, "acme-*").unwrap();

        // Act
        let covering = NamespacePattern::covering(Format::Cargo, &name);

        // Assert
        assert_eq!(covering.last(), Some(&namespace));
    }
}
