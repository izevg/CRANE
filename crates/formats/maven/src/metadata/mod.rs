//! `maven-metadata.xml` обоих уровней и выбор устаревших SNAPSHOT-сборок.
//!
//! Все функции чистые: документ выводится из фактов каталога, часы не читаются. Времена берутся
//! из `published_at`, порядок элементов задан ключом, поэтому один каталог даёт одни и те же
//! байты: документ, его сумма, `ETag` и `Last-Modified` не расходятся между запросами.

mod render;

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::time::{Duration, SystemTime};

use crane_domain::registry::{DeletePolicy, PackageName, PackageVersion, VersionState};

use crate::ecosystem::Build;
use crate::layout::FileName;
use render::{BuildsDoc, SnapshotFile, VersionsDoc};

/// Ретенция SNAPSHOT — системное удаление без окна: возраст `<= Duration::MAX` всегда истинен.
pub const RETENTION: DeletePolicy = DeletePolicy {
    window: Duration::MAX,
};

/// `g/a/maven-metadata.xml`; `None` — отдавать нечего (404).
///
/// `latest` — база версии с максимальным порядком (сборка выглядит как `X-SNAPSHOT`),
/// `release` — максимум среди версий, не являющихся timestamp-сборками.
#[must_use]
pub fn versions(package: &PackageName, served: &[PackageVersion]) -> Option<String> {
    let mut live = live(served);
    live.sort_by(|a, b| a.version.cmp(&b.version));
    let latest = base_of(live.last()?);
    // Сборки базы идут подряд: у них общий префикс ключа, и другая версия в него не попадёт.
    let mut bases: Vec<String> = live.iter().map(|v| base_of(v)).collect();
    bases.dedup();
    let release = live
        .iter()
        .rev()
        .find(|v| !v.version.is_prerelease())
        .map(|v| v.version.canonical());
    let (group, artifact) = coordinates(package);
    Some(render::versions(&VersionsDoc {
        group,
        artifact,
        latest: &latest,
        release,
        versions: &bases,
        last_updated: last_updated(&live),
    }))
}

/// `g/a/X-SNAPSHOT/maven-metadata.xml` для базы `base` (`X` без суффикса); `None` — сборок нет.
///
/// `snapshot` — последняя сборка; для каждой пары (classifier, extension) в `snapshotVersions`
/// указана самая новая сборка, где такой файл есть.
#[must_use]
pub fn builds(package: &PackageName, base: &str, served: &[PackageVersion]) -> Option<String> {
    let builds = builds_of(base, served);
    let (latest, _) = builds.last()?;
    let (group, artifact) = coordinates(package);
    let snapshot_dir = latest.base_version();
    // Идём от старых сборок к новым: поздняя запись по тому же файлу вытесняет раннюю.
    let mut newest: BTreeMap<(Option<&str>, &str), SnapshotFile> = BTreeMap::new();
    for (_, version) in &builds {
        let value = version.version.canonical();
        for asset in &version.assets {
            let Ok(file) = FileName::parse(artifact, &snapshot_dir, &asset.path) else {
                continue;
            };
            // Файл с именем другой сборки в каталоге этой версии в проекцию не попадает.
            if file.version != value {
                continue;
            }
            newest.insert(
                (file.classifier, file.extension),
                SnapshotFile {
                    classifier: file.classifier,
                    extension: file.extension,
                    value,
                    updated: version.published_at,
                },
            );
        }
    }
    let versions: Vec<&PackageVersion> = builds.iter().map(|(_, version)| *version).collect();
    Some(render::builds(&BuildsDoc {
        group,
        artifact,
        version: &snapshot_dir,
        latest,
        last_updated: last_updated(&versions),
        files: newest.into_values().collect(),
    }))
}

/// Сборки базы `base`, выходящие за `keep` самых новых по (время, номер); от старых к новым.
///
/// Возвращает только timestamp-сборки этой базы: релизы и сборки других баз не затрагиваются
/// ни при каком `keep`.
#[must_use]
pub fn stale_builds<'v>(
    base: &str,
    served: &'v [PackageVersion],
    keep: NonZeroUsize,
) -> Vec<&'v PackageVersion> {
    let builds = builds_of(base, served);
    let stale = builds.len().saturating_sub(keep.get());
    builds
        .into_iter()
        .take(stale)
        .map(|(_, version)| version)
        .collect()
}

/// Версии, которые каталог отдаёт клиентам. Каталог может вернуть и удалённые: в metadata
/// и в счёт ретенции они не входят.
fn live(served: &[PackageVersion]) -> Vec<&PackageVersion> {
    served
        .iter()
        .filter(|version| version.state == VersionState::Released)
        .collect()
}

/// Сборки базы от старых к новым.
fn builds_of<'v>(base: &str, served: &'v [PackageVersion]) -> Vec<(Build<'v>, &'v PackageVersion)> {
    let mut builds: Vec<_> = live(served)
        .into_iter()
        .filter_map(|version| Some((Build::parse(version.version.canonical())?, version)))
        .filter(|(build, _)| build.base == base)
        .collect();
    builds.sort_by_key(|(build, _)| build.sequence());
    builds
}

/// Каталог версии в layout: база для сборки, она же для остальных.
fn base_of(version: &PackageVersion) -> String {
    let canonical = version.version.canonical();
    Build::parse(canonical).map_or_else(|| canonical.to_owned(), |build| build.base_version())
}

fn last_updated(versions: &[&PackageVersion]) -> SystemTime {
    versions
        .iter()
        .map(|version| version.published_at)
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// `g.h:a` -> (`g.h`, `a`).
fn coordinates(package: &PackageName) -> (&str, &str) {
    package
        .normalized()
        .split_once(':')
        .unwrap_or((package.normalized(), ""))
}

#[cfg(test)]
mod tests;
