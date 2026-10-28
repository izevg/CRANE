use std::time::{Duration, SystemTime};

use crane_domain::registry::{
    Asset, Digests, Ecosystem, FormatMetadata, Md5, PackageName, PackageVersion, RepositoryId,
    Sha1, Sha256, Sha512, Version, VersionId, VersionState,
};
use uuid::Uuid;

use crate::ecosystem::MAVEN;

/// 2026-10-28 12:00:00 UTC.
pub(super) const T0: u64 = 1_793_188_800;
pub(super) const MINUTE: u64 = 60;
pub(super) const HOUR: u64 = 3_600;

pub(super) fn package() -> PackageName {
    MAVEN.parse_name("com.acme:core").unwrap()
}

pub(super) fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

pub(super) fn with_version(
    version: Version,
    state: VersionState,
    published: u64,
    assets: &[&str],
) -> PackageVersion {
    PackageVersion {
        id: VersionId::new(Uuid::from_u128(u128::from(published))),
        repository: RepositoryId::new(Uuid::nil()),
        package: package(),
        version,
        state,
        yanked: false,
        deprecated: None,
        published_at: at(published),
        assets: assets
            .iter()
            .map(|path| Asset {
                path: (*path).to_owned(),
                content: Digests {
                    sha256: Sha256::from([0; 32]),
                    sha512: Sha512::from([0; 64]),
                    sha1: Sha1::from([0; 20]),
                    md5: Md5::from([0; 16]),
                    size: 0,
                },
            })
            .collect(),
        metadata: FormatMetadata::new("{}".to_owned()),
    }
}

pub(super) fn released(raw: &str, published: u64, assets: &[&str]) -> PackageVersion {
    with_version(
        MAVEN.parse_version(raw).unwrap(),
        VersionState::Released,
        published,
        assets,
    )
}

pub(super) fn raws(versions: &[&PackageVersion]) -> Vec<String> {
    versions
        .iter()
        .map(|v| v.version.canonical().to_owned())
        .collect()
}
