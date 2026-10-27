use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use super::Format;

/// Ошибка разбора версии.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionError {
    #[error("некорректная версия {raw:?}: {reason}")]
    Invalid { raw: String, reason: String },
    #[error("формат {0:?} пока не поддержан")]
    Unsupported(Format),
}

/// Ключ сравнения; отдельный тип, чтобы `semver` не торчал в публичном API
/// и чтобы позже добавить схемы версий других форматов.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum VersionKey {
    SemVer(semver::Version),
}

/// Версия пакета: порядок и равенство определяет ключ формата,
/// исходная строка сохраняется для отдачи клиентам.
#[derive(Debug, Clone)]
pub struct Version {
    key: VersionKey,
    written: String,
}

impl Version {
    pub fn parse(format: Format, raw: &str) -> Result<Self, VersionError> {
        match format {
            Format::Cargo => {
                let mut parsed =
                    semver::Version::parse(raw).map_err(|e| VersionError::Invalid {
                        raw: raw.to_owned(),
                        reason: e.to_string(),
                    })?;
                // SemVer 2.0: build metadata не участвует в precedence, а `semver::Version`
                // сравнивает его лексикографически, поэтому из ключа он убирается.
                parsed.build = semver::BuildMetadata::EMPTY;
                Ok(Self {
                    key: VersionKey::SemVer(parsed),
                    written: raw.to_owned(),
                })
            }
            Format::Maven | Format::Npm | Format::PyPi | Format::NuGet => {
                Err(VersionError::Unsupported(format))
            }
        }
    }

    /// Каноническая строка ключа: по ней версия уникальна в БД.
    #[must_use]
    pub fn canonical(&self) -> String {
        match &self.key {
            VersionKey::SemVer(v) => v.to_string(),
        }
    }

    #[must_use]
    pub fn as_written(&self) -> &str {
        &self.written
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for Version {}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.written)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    fn cargo(raw: &str) -> Version {
        Version::parse(Format::Cargo, raw).unwrap()
    }

    fn hash_of(v: &Version) -> u64 {
        let mut hasher = DefaultHasher::new();
        v.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn versions_follow_semver_precedence() {
        // Arrange
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-rc.1",
            "1.0.0",
            "1.0.1",
            "1.1.0",
        ];
        let mut shuffled: Vec<Version> = ordered.iter().rev().map(|r| cargo(r)).collect();

        // Act
        shuffled.sort();

        // Assert
        let result: Vec<&str> = shuffled.iter().map(Version::as_written).collect();
        assert_eq!(result, ordered);
    }

    #[test]
    fn build_metadata_is_ignored_in_equality() {
        // Arrange
        let a = cargo("1.0.0+a");
        let b = cargo("1.0.0+b");

        // Act
        let equal = a == b;

        // Assert
        assert!(equal);
    }

    #[test]
    fn build_metadata_is_ignored_in_hash() {
        // Arrange
        let a = cargo("1.0.0+a");
        let b = cargo("1.0.0+b");

        // Act
        let (ha, hb) = (hash_of(&a), hash_of(&b));

        // Assert
        assert_eq!(ha, hb);
    }

    #[test]
    fn build_metadata_is_ignored_in_ordering() {
        // Arrange
        let a = cargo("1.0.0+z");
        let b = cargo("1.0.0+a");

        // Act
        let ordering = a.cmp(&b);

        // Assert
        assert_eq!(ordering, Ordering::Equal);
    }

    #[test]
    fn canonical_form_drops_build_metadata() {
        // Arrange
        let versions = [cargo("1.0.0+a"), cargo("1.0.0-rc.1+x")];

        // Act
        let canonical: Vec<String> = versions.iter().map(Version::canonical).collect();

        // Assert
        assert_eq!(canonical, ["1.0.0", "1.0.0-rc.1"]);
    }

    #[test]
    fn written_form_keeps_build_metadata() {
        // Arrange
        let version = cargo("1.0.0+build.5");

        // Act
        let (written, shown) = (version.as_written().to_owned(), version.to_string());

        // Assert
        assert_eq!(
            (written.as_str(), shown.as_str()),
            ("1.0.0+build.5", "1.0.0+build.5")
        );
    }

    #[test]
    fn malformed_versions_are_rejected() {
        // Arrange
        let raws = ["1.0", "v1.0.0", "01.0.0", ""];

        // Act
        let errors: Vec<_> = raws
            .iter()
            .map(|r| Version::parse(Format::Cargo, r).unwrap_err())
            .collect();

        // Assert
        assert!(
            errors
                .iter()
                .zip(raws)
                .all(|(e, raw)| matches!(e, VersionError::Invalid { raw: r, .. } if r == raw))
        );
    }

    #[test]
    fn unsupported_format_is_reported() {
        // Arrange
        let format = Format::PyPi;

        // Act
        let result = Version::parse(format, "1.0.0");

        // Assert
        assert_eq!(result.unwrap_err(), VersionError::Unsupported(Format::PyPi));
    }

    #[test]
    fn different_versions_are_not_equal() {
        // Arrange
        let (a, b, rc) = (cargo("1.0.0"), cargo("1.0.1"), cargo("1.0.0-rc.1"));

        // Act
        let (patch_differs, pre_release_differs) = (a != b, rc != a);

        // Assert
        assert!(patch_differs && pre_release_differs);
    }

    #[test]
    fn different_versions_have_different_hashes() {
        // Arrange
        let (a, b) = (cargo("1.0.0"), cargo("1.0.1"));

        // Act
        let (hash_a, hash_b) = (hash_of(&a), hash_of(&b));

        // Assert
        assert_ne!(hash_a, hash_b);
    }
}
