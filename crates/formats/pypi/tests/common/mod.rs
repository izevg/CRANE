//! Общие построители для тестов формата.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    dead_code,
    reason = "в тестах падение unwrap — это падение теста; не каждый тест использует каждый помощник"
)]

use std::time::{Duration, SystemTime};

use crane_domain::registry::{
    Asset, Ecosystem, FormatMetadata, PackageName, PackageVersion, RepositoryId, Sha256, Version,
    VersionId, VersionState,
};
use crane_format_pypi::PYPI;
use uuid::Uuid;

pub(crate) const BASE: &str = "https://crane.example/pypi/internal";

pub(crate) fn pkg(raw: &str) -> PackageName {
    PYPI.parse_name(raw).unwrap()
}

pub(crate) fn version(raw: &str) -> Version {
    PYPI.parse_version(raw).unwrap()
}

pub(crate) fn digest(byte: u8) -> Sha256 {
    Sha256::from([byte; 32])
}

pub(crate) fn asset(path: &str, byte: u8, size: u64) -> Asset {
    Asset {
        path: path.to_owned(),
        sha256: digest(byte),
        size,
    }
}

/// Версия в реестре; `at` — секунды от эпохи публикации.
pub(crate) fn stored(
    name: &str,
    raw: &str,
    at: u64,
    assets: Vec<Asset>,
    metadata: &str,
) -> PackageVersion {
    PackageVersion {
        id: VersionId::new(Uuid::from_u128(u128::from(at) + 1)),
        repository: RepositoryId::new(Uuid::from_u128(1)),
        package: pkg(name),
        version: version(raw),
        state: VersionState::Released,
        yanked: false,
        deprecated: None,
        published_at: SystemTime::UNIX_EPOCH + Duration::from_secs(at),
        assets,
        metadata: FormatMetadata::new(metadata.to_owned()),
    }
}
