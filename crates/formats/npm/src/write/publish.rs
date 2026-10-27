//! Публикация: разбор `PUT` с `_attachments`.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bytes::Bytes;
use crane_domain::registry::{Checksum, Format, FormatMetadata, PackageName, Sha1, Version};
use serde::{Deserialize, Serialize};

use super::{Attachment, Body, WriteError, parse_tag, string_field};
use crate::packument::{Fields, Raw};
use crate::route::tarball_file;
use crate::sri::{parse_sri, sha512_of};

/// Зависимости, которые ставит потребитель; `devDependencies` он не ставит, их не проверяем.
const INSTALLED_DEPENDENCIES: [&str; 3] =
    ["dependencies", "optionalDependencies", "peerDependencies"];
/// Расширения, по которым npm принимает спецификацию за файл-архив.
const FILE_SUFFIXES: [&str; 3] = [".tgz", ".tar", ".tar.gz"];
/// Ключи `_*` (служебные поля реестра) не хранятся, кроме этого.
const KEPT_UNDERSCORE: &str = "_hasShrinkwrap";
/// `publishConfig` может нести URL чужого реестра.
const DROPPED: [&str; 2] = ["deprecated", "publishConfig"];

/// Опубликованная версия, разобранная из тела запроса. Хэши не считаются: их сверяет приложение.
#[derive(PartialEq, Eq)]
pub struct NpmPublish {
    pub version: Version,
    pub file: String,
    pub tarball: Bytes,
    /// Все суммы, заявленные клиентом (sha512 обязательна); приложение обязано сверить каждую.
    pub declared: Vec<Checksum>,
    pub tag: String,
    pub metadata: FormatMetadata,
}

impl fmt::Debug for NpmPublish {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Tarball велик: в журнал идёт только длина.
        f.debug_struct("NpmPublish")
            .field("version", &self.version)
            .field("file", &self.file)
            .field("tarball_len", &self.tarball.len())
            .field("declared", &self.declared)
            .field("tag", &self.tag)
            .finish_non_exhaustive()
    }
}

/// Политика имён для новых пакетов: строже грамматики, которую читаем из upstream.
/// Хранимые hosted-пакеты появляются только через публикацию, поэтому `publish` применяет её всегда.
pub fn valid_for_new(package: &PackageName) -> Result<(), WriteError> {
    match package
        .as_written()
        .chars()
        .find(|c| c.is_ascii_uppercase() || "~'!()*".contains(*c))
    {
        Some(bad) => Err(WriteError::NewNameChar(bad)),
        None => Ok(()),
    }
}

#[derive(Deserialize)]
struct SentDist {
    integrity: Option<String>,
    shasum: Option<String>,
}

/// Что остаётся от `dist` в хранимом манифесте: URL зависит от точки монтирования и строится при рендере.
#[derive(Serialize)]
struct StoredDist<'a> {
    integrity: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    shasum: Option<&'a str>,
}

pub(super) fn publish(package: &PackageName, body: Body<'_>) -> Result<NpmPublish, WriteError> {
    valid_for_new(package)?;
    let (Some(attachments), Some(versions)) = (body.attachments, body.versions) else {
        return Err(WriteError::NotSingleVersion);
    };
    let mut versions = versions.into_iter();
    let (Some((key, manifest)), None) = (versions.next(), versions.next()) else {
        return Err(WriteError::NotSingleVersion);
    };
    let mut attachments = attachments.into_iter();
    let (Some((file, attachment)), None) = (attachments.next(), attachments.next()) else {
        return Err(WriteError::NotSingleAttachment);
    };

    let written = package.as_written();
    if body.name.as_deref() != Some(written)
        || string_field(&manifest, "name").as_deref() != Some(written)
    {
        return Err(WriteError::NameMismatch);
    }
    let version = Version::parse(Format::Npm, &key).map_err(WriteError::Version)?;
    if string_field(&manifest, "version").as_deref() != Some(key.as_ref()) {
        return Err(WriteError::VersionMismatch);
    }
    let expected_file = tarball_file(package, &version);
    // libnpmpublish (npm, pnpm) называет вложение полным именем со scope: `@scope/name-1.0.0.tgz`;
    // другие клиенты шлют базовое имя. Хранится и отдаётся всегда базовое.
    let full_name_file = format!("{written}-{}.tgz", version.as_written());
    if file != expected_file && file != full_name_file {
        return Err(WriteError::AttachmentMismatch);
    }
    let tag = single_tag(body.dist_tags, &key)?;
    check_dependencies(&manifest)?;

    let tarball = decode_attachment(&attachment)?;
    let declared = declared_sums(&manifest)?;
    let metadata = stored_metadata(manifest, &declared)?;

    Ok(NpmPublish {
        version,
        file: expected_file,
        tarball: Bytes::from(tarball),
        declared: declared.sums,
        tag,
        metadata,
    })
}

/// Вложение: base64 и, если клиент заявил длину, её совпадение с фактической.
fn decode_attachment(attachment: &Attachment<'_>) -> Result<Vec<u8>, WriteError> {
    let tarball = STANDARD
        .decode(attachment.data.as_bytes())
        .map_err(|_| WriteError::Base64)?;
    if let Some(declared) = attachment.length {
        let actual = tarball.len() as u64;
        if declared != actual {
            return Err(WriteError::LengthMismatch { declared, actual });
        }
    }
    Ok(tarball)
}

/// Суммы, заявленные клиентом в `dist`.
struct DeclaredSums {
    integrity: String,
    shasum: Option<String>,
    /// Из `integrity` (sha512 обязательна) плюс sha1 из `shasum`, если её там не было.
    sums: Vec<Checksum>,
}

fn declared_sums(manifest: &Fields<'_>) -> Result<DeclaredSums, WriteError> {
    let sent: SentDist = match manifest.get("dist") {
        Some(dist) => serde_json::from_str(dist.get())?,
        None => return Err(WriteError::MissingIntegrity),
    };
    let integrity = sent.integrity.ok_or(WriteError::MissingIntegrity)?;
    let mut sums = parse_sri(&integrity).map_err(|_| WriteError::BadIntegrity)?;
    sha512_of(&sums).ok_or(WriteError::MissingSha512)?;
    if let Some(shasum) = &sent.shasum {
        let sha1 = Checksum::Sha1(Sha1::from_hex(shasum).map_err(|_| WriteError::BadShasum)?);
        if !sums.contains(&sha1) {
            sums.push(sha1);
        }
    }
    Ok(DeclaredSums {
        integrity,
        shasum: sent.shasum,
        sums,
    })
}

/// Манифест для хранения: служебные поля убраны, `dist` сведён к суммам.
fn stored_metadata(
    mut manifest: Fields<'_>,
    declared: &DeclaredSums,
) -> Result<FormatMetadata, WriteError> {
    let stored = StoredDist {
        integrity: &declared.integrity,
        shasum: declared.shasum.as_deref(),
    };
    manifest.retain(|key, _| {
        (!key.starts_with('_') || key == KEPT_UNDERSCORE) && !DROPPED.contains(&key.as_ref())
    });
    manifest.insert("dist".into(), Raw::owned(&stored)?);
    Ok(FormatMetadata::new(serde_json::to_string(&manifest)?))
}

/// Ровно один тег, и он указывает на публикуемую версию.
fn single_tag(
    tags: Option<BTreeMap<Cow<'_, str>, Cow<'_, str>>>,
    version: &str,
) -> Result<String, WriteError> {
    let mut tags = tags.unwrap_or_default().into_iter();
    match (tags.next(), tags.next()) {
        (Some((tag, target)), None) if target == version => parse_tag(&tag),
        _ => Err(WriteError::BadDistTags),
    }
}

fn check_dependencies(manifest: &Fields<'_>) -> Result<(), WriteError> {
    for section in INSTALLED_DEPENDENCIES {
        let Some(raw) = manifest.get(section) else {
            continue;
        };
        let deps: BTreeMap<Cow<'_, str>, Cow<'_, str>> = serde_json::from_str(raw.get())?;
        if let Some((name, spec)) = deps.iter().find(|(_, spec)| !registry_spec(spec)) {
            return Err(WriteError::ForeignDependency {
                name: name.to_string(),
                spec: spec.to_string(),
            });
        }
    }
    Ok(())
}

/// Спецификация внутри реестра: диапазон или тег без `:` и `/`, либо `npm:<имя>@<диапазон или тег>`.
/// Всё остальное (URL, git, `user/repo`, file:, link:, workspace:) уводит установку мимо реестра.
fn registry_spec(spec: &str) -> bool {
    // `foo.tgz`/`foo.tar`/`foo.tar.gz` npm-package-arg читает как локальный файл, `\` — как путь Windows.
    let plain = |s: &str| {
        let lower = s.to_ascii_lowercase();
        !s.contains([':', '/', '\\']) && !FILE_SUFFIXES.iter().any(|suffix| lower.ends_with(suffix))
    };
    match spec.strip_prefix("npm:") {
        None => plain(spec),
        Some(alias) => {
            // Версия отделяется последним `@` после первого символа (первый у scope).
            let at = alias
                .get(1..)
                .and_then(|rest| rest.find('@'))
                .map(|i| i + 1);
            let (name, range) = match at {
                Some(i) => (&alias[..i], &alias[i + 1..]),
                None => (alias, ""),
            };
            PackageName::parse(Format::Npm, name).is_ok() && plain(range)
        }
    }
}
