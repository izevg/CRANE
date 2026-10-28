//! Контракт `Ecosystem`: свойства, общие для всех экосистем. Реализация даёт примеры
//! имён и версий, контракт проверяет согласованность разбора, нормализации и порядка.

use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crane_domain::registry::{Ecosystem, Format, Version};

/// Примеры, по которым проверяется экосистема.
#[derive(Debug, Clone, Copy)]
pub struct Samples {
    /// Формат, который экосистема обязана объявлять в `format()`.
    pub format: Format,
    /// Допустимые имена в разном написании (регистр, разделители).
    pub names: &'static [&'static str],
    /// Допустимые версии в строго возрастающем порядке старшинства.
    pub versions: &'static [&'static str],
}

/// Проверяет реализацию `Ecosystem` на примерах `samples`.
pub fn ecosystem_contract(eco: &dyn Ecosystem, samples: &Samples) {
    format_is_the_declared_one(eco, samples);
    canonical_version_parses_back_to_equal_version(eco, samples);
    versions_ascend_by_ord_and_precedence_along_the_chain(eco, samples);
    eq_ord_and_hash_agree_on_every_pair_of_versions(eco, samples);
    normalized_name_survives_reparse(eco, samples);
    normalize_is_idempotent(eco, samples);
    separator_is_among_accepted_separators(eco);
}

fn versions(eco: &dyn Ecosystem, samples: &Samples) -> Vec<Version> {
    samples
        .versions
        .iter()
        .map(|raw| {
            eco.parse_version(raw)
                .unwrap_or_else(|e| panic!("пример версии {raw:?} не разобран: {e}"))
        })
        .collect()
}

fn hash_of(version: &Version) -> u64 {
    let mut hasher = DefaultHasher::new();
    version.hash(&mut hasher);
    hasher.finish()
}

fn format_is_the_declared_one(eco: &dyn Ecosystem, samples: &Samples) {
    // Arrange
    let expected = samples.format;

    // Act
    let actual = eco.format();

    // Assert
    assert_eq!(
        actual, expected,
        "format(): экосистема объявляет чужой формат"
    );
}

fn canonical_version_parses_back_to_equal_version(eco: &dyn Ecosystem, samples: &Samples) {
    // Arrange
    let parsed = versions(eco, samples);

    // Act
    let reparsed: Vec<_> = parsed
        .iter()
        .map(|v| eco.parse_version(v.canonical()))
        .collect();

    // Assert
    let expected: Vec<_> = parsed.into_iter().map(Ok).collect();
    assert_eq!(
        reparsed, expected,
        "parse_version: каноническая строка должна разбираться в равную версию"
    );
}

fn versions_ascend_by_ord_and_precedence_along_the_chain(eco: &dyn Ecosystem, samples: &Samples) {
    // Arrange
    let parsed = versions(eco, samples);

    // Act
    let steps: Vec<_> = parsed
        .windows(2)
        .map(|pair| (pair[0].cmp(&pair[1]), pair[0].precedence(&pair[1])))
        .collect();

    // Assert
    let expected = vec![(Ordering::Less, Ordering::Less); steps.len()];
    assert_eq!(
        steps, expected,
        "порядок версий не совпал с заявленной цепочкой примеров"
    );
}

fn eq_ord_and_hash_agree_on_every_pair_of_versions(eco: &dyn Ecosystem, samples: &Samples) {
    // Arrange
    let parsed = versions(eco, samples);
    let pairs: Vec<(&Version, &Version)> = parsed
        .iter()
        .flat_map(|a| parsed.iter().map(move |b| (a, b)))
        .collect();

    // Act
    let agreed: Vec<bool> = pairs
        .iter()
        .map(|(a, b)| {
            let equal = a == b;
            equal == (a.cmp(b) == Ordering::Equal) && (!equal || hash_of(a) == hash_of(b))
        })
        .collect();

    // Assert
    assert_eq!(
        agreed,
        vec![true; pairs.len()],
        "Eq, Ord и Hash версий несогласованы: a == b ⇔ cmp Equal, равные версии — равные хеши"
    );
}

fn normalized_name_survives_reparse(eco: &dyn Ecosystem, samples: &Samples) {
    // Arrange
    let names = samples.names;

    // Act
    let pairs: Vec<_> = names
        .iter()
        .map(|raw| {
            let first = eco.parse_name(raw).map(|n| n.normalized().to_owned());
            let again = first
                .clone()
                .and_then(|norm| eco.parse_name(&norm).map(|n| n.normalized().to_owned()));
            (first, again)
        })
        .collect();

    // Assert
    let expected: Vec<_> = pairs
        .iter()
        .map(|(first, _)| (first.clone(), first.clone()))
        .collect();
    assert_eq!(
        pairs, expected,
        "parse_name(normalized) должен сохранять нормализованную форму"
    );
}

fn normalize_is_idempotent(eco: &dyn Ecosystem, samples: &Samples) {
    // Arrange
    let names = samples.names;

    // Act
    let pairs: Vec<_> = names
        .iter()
        .map(|raw| {
            let once = eco.normalize(raw);
            let twice = eco.normalize(&once);
            (once, twice)
        })
        .collect();

    // Assert
    let expected: Vec<_> = pairs
        .iter()
        .map(|(once, _)| (once.clone(), once.clone()))
        .collect();
    assert_eq!(
        pairs, expected,
        "normalize: повторное применение меняет результат"
    );
}

fn separator_is_among_accepted_separators(eco: &dyn Ecosystem) {
    // Arrange
    let accepted = eco.accepted_separators();

    // Act
    let separator = eco.separator();

    // Assert
    assert!(
        accepted.contains(&separator),
        "separator() {separator:?} должен входить в accepted_separators() {accepted:?}"
    );
}
