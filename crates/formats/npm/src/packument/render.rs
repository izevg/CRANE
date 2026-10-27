//! Сборка документов: из ответа upstream и из состояния hosted-репозитория.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crane_domain::registry::{Format, PackageName, PackageVersion, Version, VersionId};

use super::model::Packument;
use super::project::abbreviate;
use super::{DocError, Fields, Raw, TarballBase, View};
use crate::route::tarball_file;

const LATEST: &str = "latest";

fn finish(mut doc: Packument<'_>, view: View) -> Result<Vec<u8>, DocError> {
    if view == View::Abbreviated {
        abbreviate(&mut doc)?;
    }
    doc.to_vec()
}

/// `dist.tarball` строится только из точки монтирования, имени запрошенного пакета и разобранной версии:
/// ни один URL из upstream в вывод не попадает.
fn tarball_url(base: TarballBase<'_>, package: &PackageName, version: &Version) -> String {
    format!(
        "{}/{}/-/{}",
        base.0.trim_end_matches('/'),
        package.as_written(),
        tarball_file(package, version)
    )
}

/// Подменяет `dist.tarball`; `None`, если `dist` отсутствует или не объект.
fn with_tarball(dist: Option<&Raw<'_>>, url: &str) -> Result<Option<Raw<'static>>, DocError> {
    let Some(mut fields) = dist.and_then(Raw::as_object) else {
        return Ok(None);
    };
    fields.insert("tarball".into(), Raw::owned(&url)?);
    Ok(Some(Raw::owned(&fields)?))
}

/// Перерисовывает packument upstream запрошенного `package` для нашей точки монтирования.
///
/// Версии с ключом, не разбирающимся как версия, с чужим `name` или без `dist` отбрасываются:
/// мусор не превращается в ссылку на скачивание.
pub fn render_upstream(
    raw: &[u8],
    package: &PackageName,
    view: View,
    base: TarballBase<'_>,
) -> Result<Vec<u8>, DocError> {
    let mut doc = Packument::parse(raw)?;
    let name = doc
        .fields
        .get("name")
        .and_then(Raw::as_string)
        .ok_or(DocError::MissingName)?;
    if name != package.as_written() {
        return Err(DocError::NameMismatch);
    }
    let versions = std::mem::take(&mut doc.versions);
    for (key, mut fields) in versions {
        let Ok(version) = Version::parse(Format::Npm, &key) else {
            continue;
        };
        let foreign_name = fields
            .get("name")
            .is_some_and(|n| n.as_string().as_deref() != Some(package.as_written()));
        if foreign_name {
            continue;
        }
        let url = tarball_url(base, package, &version);
        let Some(dist) = with_tarball(fields.get("dist"), &url)? else {
            continue;
        };
        fields.insert("dist".into(), dist);
        doc.versions.insert(key, fields);
    }
    retain_present_tags(&mut doc)?;
    finish(doc, view)
}

/// Тег на отброшенную версию ломает `npm i pkg@tag`; `latest` для upstream не придумываем.
fn retain_present_tags(doc: &mut Packument<'_>) -> Result<(), DocError> {
    let Some(tags) = doc.fields.get("dist-tags").and_then(Raw::as_object) else {
        return Ok(());
    };
    let present: BTreeMap<&str, String> = tags
        .iter()
        .filter_map(|(tag, target)| Some((tag.as_ref(), target.as_string()?)))
        .filter(|(_, version)| doc.versions.contains_key(version.as_str()))
        .collect();
    let filtered = Raw::owned(&present)?;
    doc.fields.insert("dist-tags".into(), filtered);
    Ok(())
}

fn hosted_version<'a>(
    package: &PackageName,
    stored: &'a PackageVersion,
    base: TarballBase<'_>,
) -> Result<Fields<'a>, DocError> {
    let mut fields: Fields<'a> = serde_json::from_str(stored.metadata.as_str())?;
    let url = tarball_url(base, package, &stored.version);
    let dist = match with_tarball(fields.get("dist"), &url)? {
        Some(dist) => dist,
        None => Raw::owned(&BTreeMap::from([("tarball", url.as_str())]))?,
    };
    fields.insert("dist".into(), dist);
    fields.insert(
        "_id".into(),
        Raw::owned(&format!("{package}@{}", stored.version))?,
    );
    fields.remove("deprecated");
    if let Some(message) = &stored.deprecated {
        fields.insert("deprecated".into(), Raw::owned(message)?);
    }
    Ok(fields)
}

/// `latest`, если тега нет: старшая версия без pre-release, а при их отсутствии — старшая вообще.
fn fallback_latest(versions: &[PackageVersion]) -> Option<&Version> {
    let highest = |only_releases: bool| {
        versions
            .iter()
            .map(|v| &v.version)
            .filter(|v| !only_releases || !v.is_prerelease())
            .max()
    };
    highest(true).or_else(|| highest(false))
}

fn rfc3339(at: SystemTime) -> String {
    humantime::format_rfc3339_millis(at).to_string()
}

/// Теги, которые видит клиент: тег на версию, которой нет среди отдаваемых, отбрасывается,
/// а отсутствующий `latest` выводится (старшая версия без pre-release, иначе старшая вообще).
/// Единственное место этого правила: его используют и packument, и ответ `GET dist-tags`.
#[must_use]
pub fn effective_tags<'a>(
    versions: &'a [PackageVersion],
    tags: &'a [(String, VersionId)],
) -> BTreeMap<&'a str, &'a Version> {
    let mut effective: BTreeMap<&str, &Version> = tags
        .iter()
        .filter_map(|(tag, id)| {
            let stored = versions.iter().find(|v| v.id == *id)?;
            Some((tag.as_str(), &stored.version))
        })
        .collect();
    if !effective.contains_key(LATEST)
        && let Some(latest) = fallback_latest(versions)
    {
        effective.insert(LATEST, latest);
    }
    effective
}

/// Собирает packument hosted-репозитория. `versions` — уже отдаваемые (served) версии, непустые:
/// пустой список — 404 вызывающего, а не документ.
pub fn render_hosted(
    package: &PackageName,
    versions: &[PackageVersion],
    tags: &[(String, VersionId)],
    view: View,
    base: TarballBase<'_>,
) -> Result<Vec<u8>, DocError> {
    if versions.is_empty() {
        return Err(DocError::NoVersions);
    }
    let mut doc = Packument::default();
    for stored in versions {
        doc.versions.insert(
            Cow::Borrowed(stored.version.as_written()),
            hosted_version(package, stored, base)?,
        );
    }

    let dist_tags: BTreeMap<&str, &str> = effective_tags(versions, tags)
        .into_iter()
        .map(|(tag, version)| (tag, version.as_written()))
        .collect();

    let created = versions.iter().map(|v| v.published_at).min();
    let modified = versions.iter().map(|v| v.published_at).max();
    let (Some(created), Some(modified)) = (created, modified) else {
        return Err(DocError::NoVersions);
    };
    let mut time: BTreeMap<&str, String> = versions
        .iter()
        .map(|v| (v.version.as_written(), rfc3339(v.published_at)))
        .collect();
    time.insert("created", rfc3339(created));
    time.insert("modified", rfc3339(modified));

    let name = package.as_written();
    doc.fields.insert("_id".into(), Raw::owned(&name)?);
    doc.fields.insert("name".into(), Raw::owned(&name)?);
    doc.fields
        .insert("dist-tags".into(), Raw::owned(&dist_tags)?);
    doc.fields.insert("time".into(), Raw::owned(&time)?);
    if view == View::Full {
        // `_rev` = число отдаваемых версий и время последней публикации. Устаревание и теги его не меняют:
        // сервер `_rev` при записи не сверяет (PUT ничего не разрушает), клиенту он нужен лишь как
        // детерминированная метка документа, и этой пары для неё достаточно.
        let millis = modified
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis();
        doc.fields.insert(
            "_rev".into(),
            Raw::owned(&format!("{}-{millis}", versions.len()))?,
        );
    }
    finish(doc, view)
}
