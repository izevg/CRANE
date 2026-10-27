//! Знание registry.npmjs.org для proxy-репозитория: хост, политики и поиск tarball в packument.
//!
//! Ключи кэша packument и tarball под одним `RepositoryId` не пересекаются: путь packument — имя
//! без сегмента `/-/` (`react`, `@scope%2fname`), путь tarball всегда содержит `/-/`.

use std::borrow::Cow;
use std::num::NonZeroU32;
use std::time::Duration;

use crane_domain::proxy::{MetadataPolicy, NegativePolicy, PathError, UpstreamPath};
use crane_domain::registry::{Checksum, DeletePolicy, PackageName, Sha1};
use serde::Deserialize;

use crate::dist::{parse_sri, sha512_of};
use crate::route::tarball_version;

pub const REGISTRY_HOST: &str = "registry.npmjs.org";

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
/// N = 3 из 11.1. Через `MIN + 2`, потому что `unwrap` в рабочем коде запрещён, а `new(3)` возвращает `Option`.
const ADAPTIVE_AFTER: NonZeroU32 = NonZeroU32::MIN.saturating_add(2);

/// Строка 11.1 «npm packument»: TTL 5 мин, stale-while-revalidate 5 мин, stale-if-error 7 дней,
/// negative 1 мин с ростом до 1 ч; N = 3.
pub const PACKUMENT: MetadataPolicy = MetadataPolicy {
    ttl: Duration::from_secs(5 * MINUTE),
    revalidate_window: Duration::from_secs(5 * MINUTE),
    stale_if_error: Duration::from_secs(7 * DAY),
    adaptive_after: ADAPTIVE_AFTER,
    negative: NegativePolicy {
        ttl: Duration::from_secs(MINUTE),
        max: Duration::from_secs(HOUR),
    },
};

/// Окно снятия версии (`npm unpublish`): 72 часа с публикации, док. 09.
pub const UNPUBLISH: DeletePolicy = DeletePolicy {
    window: Duration::from_secs(72 * HOUR),
};

/// Принимать ли tarball, у которого есть только sha1 (`dist.shasum`), без sha512 в `dist.integrity`.
///
/// Нет: sha1 подделывается коллизиями, и «проверенный» им архив ничего не гарантирует. Старые версии
/// без SRI в proxy недоступны сознательно; включать только решением владельца.
pub const ACCEPT_SHA1_ONLY: bool = false;

/// Путь packument у upstream: `name` или `@scope%2fname` (один сегмент, как ждёт реестр).
pub fn packument_path(package: &PackageName) -> Result<UpstreamPath, PathError> {
    UpstreamPath::parse(&package.as_written().replace('/', "%2f"))
}

/// Где лежит tarball версии и какой суммы он обязан быть.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tarball {
    pub path: UpstreamPath,
    pub expected: Checksum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TarballError {
    #[error("такой версии или файла нет в packument")]
    NotListed,
    #[error("у версии нет sha512 в dist.integrity: проверить tarball нечем")]
    WeakIntegrity,
}

/// Читаемая часть packument; остальное игнорируется (совместимость вперёд).
#[derive(Deserialize)]
struct Doc<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow, default)]
    versions: std::collections::BTreeMap<Cow<'a, str>, Entry<'a>>,
}

#[derive(Deserialize)]
struct Entry<'a> {
    #[serde(borrow)]
    dist: Option<Dist<'a>>,
}

#[derive(Deserialize)]
struct Dist<'a> {
    #[serde(borrow)]
    integrity: Option<Cow<'a, str>>,
    #[serde(borrow)]
    shasum: Option<Cow<'a, str>>,
}

/// Tarball версии по телу packument upstream.
///
/// Путь строится из запрошенного имени и файла, подтверждённых документом (`name` обязан совпасть),
/// а не из `dist.tarball`: ни один URL upstream в запрос не попадает.
pub fn tarball(raw: &[u8], package: &PackageName, file: &str) -> Result<Tarball, TarballError> {
    let version = tarball_version(package, file).ok_or(TarballError::NotListed)?;
    let doc: Doc<'_> = serde_json::from_slice(raw).map_err(|_| TarballError::NotListed)?;
    if doc.name != package.as_written() {
        return Err(TarballError::NotListed);
    }
    let dist = doc
        .versions
        .get(version.as_written())
        .ok_or(TarballError::NotListed)?
        .dist
        .as_ref();
    let sums = dist
        .and_then(|d| d.integrity.as_deref())
        .and_then(|integrity| parse_sri(integrity).ok())
        .unwrap_or_default();
    let expected = match sha512_of(&sums) {
        Some(sum) => sum,
        None if ACCEPT_SHA1_ONLY => dist
            .and_then(|d| d.shasum.as_deref())
            .and_then(|hex| Sha1::from_hex(hex).ok())
            .map(Checksum::Sha1)
            .ok_or(TarballError::WeakIntegrity)?,
        None => return Err(TarballError::WeakIntegrity),
    };
    let path = UpstreamPath::parse(&format!("{}/-/{file}", package.as_written()))
        .map_err(|_| TarballError::NotListed)?;
    Ok(Tarball { path, expected })
}
