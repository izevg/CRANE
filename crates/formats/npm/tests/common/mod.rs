//! Общие построители для тестов формата.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    dead_code,
    reason = "в тестах падение unwrap — это падение теста; не каждый тест использует каждый помощник"
)]

use std::time::{Duration, SystemTime};

use crane_domain::registry::{
    Format, FormatMetadata, PackageName, PackageVersion, RepositoryId, Version, VersionId,
    VersionState,
};
use uuid::Uuid;

pub(crate) fn pkg(raw: &str) -> PackageName {
    PackageName::parse(Format::Npm, raw).unwrap()
}

pub(crate) fn version(raw: &str) -> Version {
    Version::parse(Format::Npm, raw).unwrap()
}

pub(crate) fn version_id(n: u128) -> VersionId {
    VersionId::new(Uuid::from_u128(n))
}

/// Версия в реестре; `n` — идентификатор, `at` — секунды от эпохи публикации.
pub(crate) fn stored(
    name: &str,
    raw: &str,
    n: u128,
    at: u64,
    deprecated: Option<&str>,
    metadata: &str,
) -> PackageVersion {
    PackageVersion {
        id: version_id(n),
        repository: RepositoryId::new(Uuid::from_u128(1)),
        package: pkg(name),
        version: version(raw),
        state: VersionState::Released,
        yanked: false,
        deprecated: deprecated.map(str::to_owned),
        published_at: SystemTime::UNIX_EPOCH + Duration::from_secs(at),
        assets: Vec::new(),
        metadata: FormatMetadata::new(metadata.to_owned()),
    }
}
