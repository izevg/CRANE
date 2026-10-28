//! Паттерн namespace: разбор, нормализация и список паттернов, покрывающих имя.

use super::super::{Ecosystem, Format, NameError, PackageName};

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
    /// Разбирает паттерн из конфигурации по правилам экосистемы.
    ///
    /// Префикс обязан оканчиваться разделителем: `acme-*` покрывает `acme-core`, но не
    /// `acmecorp`, иначе чужой пакет с общим началом имени оказался бы «внутренним».
    pub fn parse(eco: &dyn Ecosystem, raw: &str) -> Result<Self, PatternError> {
        if raw.is_empty() {
            return Err(PatternError::Empty);
        }
        if let Some(bad) = raw.chars().find(|c| !c.is_ascii_graphic()) {
            return Err(PatternError::Charset(bad));
        }
        match raw.matches(WILDCARD).count() {
            0 => Self::parse_exact(eco, raw),
            1 if raw.ends_with(WILDCARD) => Self::parse_prefix(eco, raw),
            _ => Err(PatternError::Wildcard),
        }
    }

    fn parse_exact(eco: &dyn Ecosystem, raw: &str) -> Result<Self, PatternError> {
        Ok(Self(eco.parse_name(raw)?.normalized().to_owned()))
    }

    fn parse_prefix(eco: &dyn Ecosystem, raw: &str) -> Result<Self, PatternError> {
        // Срез по ASCII-символу `*` в конце: граница символа гарантирована.
        let head = raw.trim_end_matches(WILDCARD);
        match eco.format() {
            Format::Npm => Self::parse_scope(head),
            Format::Maven | Format::Cargo | Format::PyPi | Format::NuGet => {
                Self::parse_delimited(eco, head)
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

    fn parse_delimited(eco: &dyn Ecosystem, head: &str) -> Result<Self, PatternError> {
        let Some(last) = head.chars().last() else {
            return Err(PatternError::EmptyBase);
        };
        if eco.format() == Format::Maven && head.contains(':') {
            return Err(PatternError::GroupId);
        }
        if !eco.accepted_separators().contains(&last) {
            return Err(PatternError::Boundary);
        }
        let normalized = eco.normalize(head);
        let sep = eco.separator();
        let base = normalized.strip_suffix(sep).unwrap_or(&normalized);
        if base.is_empty() {
            return Err(PatternError::EmptyBase);
        }
        // Двойной разделитель делает бессмысленную базу `acme_` и паттерн, которому
        // не соответствует ни одно имя, написанное оператором «на глаз».
        if base.ends_with(sep) {
            return Err(PatternError::Boundary);
        }
        let raw_base = head.get(..head.len() - last.len_utf8()).unwrap_or_default();
        eco.parse_name(raw_base)?;
        Ok(Self(format!("{normalized}{WILDCARD}")))
    }

    /// Точное имя как паттерн.
    #[must_use]
    pub fn exact(name: &PackageName) -> Self {
        Self(name.normalized().to_owned())
    }

    /// Все паттерны, которые покрыли бы имя, от самого специфичного к самому общему.
    #[must_use]
    pub fn covering(eco: &dyn Ecosystem, name: &PackageName) -> Vec<Self> {
        covering_of(eco, name.normalized())
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

/// То же, что [`NamespacePattern::covering`], над нормализованной строкой.
fn covering_of(eco: &dyn Ecosystem, normalized: &str) -> Vec<NamespacePattern> {
    let mut patterns = vec![NamespacePattern(normalized.to_owned())];
    match eco.format() {
        Format::Npm => patterns.extend(scope_prefix(normalized)),
        Format::Maven => {
            // Имя без `:` не принадлежит никакому groupId: покрыть его может только точный паттерн.
            if let Some((group, _)) = normalized.split_once(':') {
                patterns.extend(group_prefixes(group));
            }
        }
        Format::Cargo | Format::PyPi | Format::NuGet => {
            patterns.extend(prefixes(normalized, eco.separator()));
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
#[path = "tests/pattern.rs"]
mod tests;
